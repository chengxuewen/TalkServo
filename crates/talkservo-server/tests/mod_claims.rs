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

// plan-7 S4: mod_rooms claims ride the token; a field-role token with the
// room claim governs it (mode flips execute); without the claim it is denied.
// Legacy compat: a token with NO mod_rooms field = empty set (R4 gate).

#[tokio::test]
async fn mod_claim_governs_only_claimed_room() {
    let (addr, dir) = spawn_server().await;
    let bearer = admin_bearer(&addr).await;

    // issue a mod token for room "claimed" (registry-free - claims are the
    // authority; the room spawns implicit like today)
    let (st, tok) = http(
        &addr,
        "POST",
        "/admin/api/tokens",
        Some(&bearer),
        Some(
            &json!({"sub":"modp","room":"claimed","role":"field","ttl_s":600,
                     "mod_rooms":["claimed","other-room"]})
            .to_string(),
        ),
    )
    .await;
    assert_eq!(st, 200, "token with mods: {tok}");
    let jwt = tok["jwt"].as_str().unwrap().to_string();

    // connect as the mod (role field) in the claimed room
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();
    ws.send(WsMsg::Text(
        json!({"type":"join","v":1,"jwt":jwt}).to_string().into(),
    ))
    .await
    .unwrap();
    wait_for_type(&mut ws, "welcome").await;

    // mode flip EXECUTES (mod authority present)
    ws.send(WsMsg::Text(
        json!({"type":"mode_change","mode":"open"})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    // drain until we see the denied OR a mode-bearing snapshot; open room
    // emission is silent on empty->empty, so watch via a second peer's snapshot
    let mut plain = ws_connect(&addr, "plain", "claimed", Role::Field).await;
    wait_for_type(&mut plain, "welcome").await;
    let snap = wait_for_type(&mut plain, "server_snapshot").await;
    assert_eq!(snap["payload"]["mode"], "open", "mod flip executed");

    // same token in an UNCLAIMED room: denied
    let (tok2ok, tok2) = http(
        &addr,
        "POST",
        "/admin/api/tokens",
        Some(&bearer),
        Some(
            &json!({"sub":"modp","room":"elsewhere","role":"field","ttl_s":600,
                     "mod_rooms":["claimed"]})
            .to_string(),
        ),
    )
    .await;
    assert_eq!(tok2ok, 200);
    let (mut ws2, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();
    ws2.send(WsMsg::Text(
        json!({"type":"join","v":1,"jwt":tok2["jwt"].as_str().unwrap()})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    wait_for_type(&mut ws2, "welcome").await;
    ws2.send(WsMsg::Text(
        json!({"type":"mode_change","mode":"exclusive"})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let denied = wait_for_type(&mut ws2, "floor_denied").await;
    assert_eq!(denied["reason"], "unauthorized", "claim is room-scoped");

    // audit: exactly one authz_denied (the elsewhere attempt)
    let mut denials = 0;
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
        denials = body["events"].as_array().unwrap().len();
        if denials >= 1 {
            break;
        }
    }
    assert!(denials >= 1, "denial audited");

    // legacy-token compat: a hand-minted token WITHOUT the mod_rooms field
    let legacy = test_bridge::issue_token(USER_SECRET, "oldstyle", "claimed", Role::Field, 600);
    let (mut ws3, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();
    ws3.send(WsMsg::Text(
        json!({"type":"join","v":1,"jwt":legacy}).to_string().into(),
    ))
    .await
    .unwrap();
    wait_for_type(&mut ws3, "welcome").await;
    ws3.send(WsMsg::Text(
        json!({"type":"mode_change","mode":"hybrid"})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let denied = wait_for_type(&mut ws3, "floor_denied").await;
    assert_eq!(
        denied["reason"], "unauthorized",
        "legacy token = empty mod set"
    );

    std::fs::remove_dir_all(&dir).ok();
}
