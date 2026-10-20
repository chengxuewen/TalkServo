#![cfg(feature = "stub-media")] // test_bridge is stub-gated (ws_flow convention)
// plan-7 S2: the rooms registry drives spawn mode + join lock; the live poll
// answers under its renamed path; delete removes the row AND evicts members
// (audit trail records the forced leaves). max_peers is accepted-but-unenforced
// today (open item: member cap never existed pre-S2; FloorLimits.max_peers is
// dead in core) - asserted as passthrough only.
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use talkservo_core::wire::Role;
use talkservo_server::test_bridge;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message as WsMsg;

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const USER_SECRET: &str = "rooms-user-secret";
const ADMIN_SECRET: &str = "rooms-admin-secret";
const PASSWORD: &str = "rooms-pw";

async fn http(
    addr: &str,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: Option<&str>,
) -> (u16, Value) {
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: tests\r\nConnection: close\r\n");
    if let Some(b) = bearer {
        req.push_str(&format!("Authorization: Bearer {b}\r\n"));
    }
    match body {
        Some(b) => req.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{b}",
            b.len()
        )),
        None => req.push_str("\r\n"),
    }
    let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
    stream.write_all(req.as_bytes()).await.expect("write");
    stream.flush().await.expect("flush");
    let mut raw = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.read_to_end(&mut raw),
    )
    .await
    .expect("http guard")
    .expect("read");
    let text = String::from_utf8_lossy(&raw).to_string();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status");
    let json = text
        .split("\r\n\r\n")
        .nth(1)
        .and_then(|b| serde_json::from_str(b).ok())
        .unwrap_or(Value::Null);
    (status, json)
}

async fn spawn_server() -> (String, std::path::PathBuf) {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ts-rooms-it-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("etc")).unwrap();
    let mut config = test_bridge::config_for_test(USER_SECRET);
    config.admin_secret = Some(ADMIN_SECRET.to_string());
    config.admin_password_hash = Some(talkservo_server::kdf::hash_password_with_iters(
        PASSWORD, 4096,
    ));
    config.accounts_file = Some(dir.join("etc/accounts.yaml"));
    config.rooms_file = Some(dir.join("etc/rooms.yaml"));
    config.audit_log = Some(dir.join("logs/audit.jsonl"));
    let app = test_bridge::build_router(test_bridge::test_app(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, dir)
}

async fn admin_bearer(addr: &str) -> String {
    let (st, login) = http(
        addr,
        "POST",
        "/admin/api/login",
        None,
        Some(&json!({"user": "admin", "password": PASSWORD}).to_string()),
    )
    .await;
    assert_eq!(st, 200, "login: {login}");
    login["jwt"].as_str().unwrap().to_string()
}

async fn ws_connect(addr: &str, sub: &str, room: &str, role: Role) -> Ws {
    let jwt = test_bridge::issue_token(USER_SECRET, sub, room, role, 600);
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();
    ws.send(WsMsg::Text(
        json!({"type": "join", "v": 1, "jwt": jwt})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    ws
}

async fn next_msg(ws: &mut Ws) -> Value {
    for _ in 0..24 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
            .await
            .expect("frame timeout")
            .expect("open")
            .expect("text");
        if let Ok(v) = serde_json::from_str::<Value>(frame.to_text().unwrap()) {
            return v;
        }
    }
    panic!("no message");
}

async fn wait_for_type(ws: &mut Ws, want: &str) -> Value {
    for _ in 0..24 {
        let v = next_msg(ws).await;
        if v["type"] == want {
            return v;
        }
    }
    panic!("never saw {want}");
}

// plan-7 S3: ModeChange/MuteSet are Dispatch-gated (Rule 10, modules/01).
// Field sends: denied FloorDenied{unauthorized} + authz_denied audit line;
// Dispatch sends: executed; floor state untouched by denials (R8-style).

#[tokio::test]
async fn field_cannot_flip_mode_but_dispatch_can() {
    let (addr, dir) = spawn_server().await;
    let mut field = ws_connect(&addr, "fieldy", "authz", Role::Field).await;
    let mut chief = ws_connect(&addr, "chief", "authz", Role::Dispatch).await;

    // field attempts ModeChange -> FloorDenied{unauthorized}, no mode delta
    field
        .send(WsMsg::Text(
            json!({"type": "mode_change", "mode": "open"})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let denied = wait_for_type(&mut field, "floor_denied").await;
    assert_eq!(denied["reason"], "unauthorized");

    // dispatch attempt succeeds (wait on chief's mirror for the mode delta)
    chief
        .send(WsMsg::Text(
            json!({"type": "mode_change", "mode": "open"})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    wait_for_type(&mut chief, "server_snapshot").await;

    // audit: exactly the denial is recorded; the mode_change is NOT (it is a
    // legit operation - relayed via mode_change events only after S1's relay
    // at apply; here we assert the denial kind exists)
    let bearer = admin_bearer(&addr).await;
    let mut saw_denial = false;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (st, body) = http(
            &addr,
            "GET",
            "/admin/api/audit?event=authz_denied",
            Some(&bearer),
            None,
        )
        .await;
        assert_eq!(st, 200);
        if !body["events"].as_array().unwrap().is_empty() {
            let e = &body["events"].as_array().unwrap()[0];
            assert_eq!(e["actor"], "fieldy");
            assert_eq!(e["room"], "authz");
            saw_denial = true;
            break;
        }
    }
    assert!(saw_denial, "authz_denied line recorded");

    drop(field);
    drop(chief);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn field_cannot_mute_anyone_dispatch_mutes() {
    let (addr, dir) = spawn_server().await;
    let mut a = ws_connect(&addr, "aa", "muteroom", Role::Field).await;
    let mut b = ws_connect(&addr, "bb", "muteroom", Role::Field).await;
    let mut chief = ws_connect(&addr, "chief", "muteroom", Role::Dispatch).await;

    a.send(WsMsg::Text(
        json!({"type": "floor_request", "priority": 0, "preempt": false})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    wait_for_type(&mut a, "floor_granted").await;

    // grant bb first (core MuteSet: unknown target = no-op; R8 documented)
    b.send(WsMsg::Text(
        json!({"type": "floor_request", "priority": 0, "preempt": false})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    wait_for_type(&mut b, "floor_granted").await;

    // peer (field) mutes -> denied
    a.send(WsMsg::Text(
        json!({"type": "mute_set", "peer": "bb", "on": true})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let denied = wait_for_type(&mut a, "floor_denied").await;
    assert_eq!(denied["reason"], "unauthorized");

    // dispatch mutes -> executed
    chief
        .send(WsMsg::Text(
            json!({"type": "mute_set", "peer": "bb", "on": true})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    // observe via the admin live poll (AdminInspect carries the muted set;
    // role-scoped WS delivery of muted tags differs per payload shape and the
    // poll is the canonical observer per plan-5)
    let bearer = admin_bearer(&addr).await;
    let mut muted = false;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (st, body) = http(&addr, "GET", "/admin/api/rooms/live", Some(&bearer), None).await;
        assert_eq!(st, 200);
        if let Some(room) = body
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["room"] == "muteroom")
            && room
                .get("muted")
                .and_then(|m| m.as_array())
                .is_some_and(|l| l.iter().any(|v| v == "bb"))
        {
            muted = true;
            break;
        }
    }
    assert!(muted, "bb muted in AdminInspect after dispatch mute");

    drop(a);
    drop(b);
    drop(chief);
    std::fs::remove_dir_all(&dir).ok();
}
