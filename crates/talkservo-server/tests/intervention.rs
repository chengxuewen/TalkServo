#![cfg(feature = "stub-media")]
// test_bridge is stub-gated (ws_flow convention)
// plan-7 S6: the intervention kit. kick = console-driven removal (peer's
// socket closes, survivors see peer_left kind=removed, audit line carries the
// reason + console actor); force-release routes the peer's own Release through
// the authorized path (PIT-21 rebroadcast proves grants shrank); operator
// tier may intervene, user tokens cannot.
#![allow(dead_code)] // shared harness copy

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use talkservo_core::wire::Role;
use talkservo_server::test_bridge;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message as WsMsg;

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const USER_SECRET: &str = "interv-user-secret";
const ADMIN_SECRET: &str = "interv-admin-secret";
const PASSWORD: &str = "interv-pw";

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
        "ts-interv-{}-{}",
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

async fn join(addr: &str, sub: &str, room: &str, role: Role) -> Ws {
    let jwt = test_bridge::issue_token(USER_SECRET, sub, room, role, 3600);
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
    // drain J-burst quiet-period
    for _ in 0..6 {
        if tokio::time::timeout(std::time::Duration::from_millis(400), ws.next())
            .await
            .is_err()
        {
            break;
        }
    }
    ws
}

async fn recv_type(ws: &mut Ws, want: &str) -> Value {
    for _ in 0..24 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
            .await
            .expect("frame timeout")
            .expect("open")
            .expect("text");
        let v: Value = serde_json::from_str(frame.to_text().unwrap()).unwrap();
        if v["type"] == want {
            return v;
        }
    }
    panic!("never saw {want}");
}

#[tokio::test]
async fn kick_closes_target_marks_removed_and_audits_reason() {
    let (addr, dir) = spawn_server().await;
    let bearer = admin_bearer(&addr).await;

    let mut a = join(&addr, "alice", "ik", Role::Field).await;
    let mut b = join(&addr, "bob", "ik", Role::Field).await;

    // console kick bob (operator tier via admin token here; reason set)
    let (st, resp) = http(
        &addr,
        "DELETE",
        "/admin/api/rooms/ik/peers/bob",
        Some(&bearer),
        Some(&json!({"reason": "abuse of open mic"}).to_string()),
    )
    .await;
    assert_eq!(st, 204, "kick: {resp}");

    // survivor (alice) sees the kind-marked left frame (R2: NO reason/actor)
    let left = recv_type(&mut a, "peer_left").await;
    assert_eq!(left["peer"], "bob");
    assert_eq!(left["kind"], "removed", "additive marker reaches survivors");

    // R7-b: the kick must be TERMINAL for the client, not a network drop. Any other
    // close code makes packages/client/src/signal.ts reconnect, and a peer holding a
    // still-valid token walks back in within seconds — which is exactly what the
    // cross-client e2e row caught (PIT-26).
    let code = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match b.next().await {
                Some(Ok(WsMsg::Close(Some(f)))) => return u16::from(&f.code),
                Some(Ok(_)) => continue,
                other => panic!("kicked socket ended without a close frame: {other:?}"),
            }
        }
    })
    .await
    .expect("the kicked connection closes promptly");
    assert_eq!(
        code,
        talkservo_server::ws::CLOSE_REMOVED,
        "kick must answer with the terminal close code, not 1000"
    );

    // audit: kick line attributed to the console account, reason carried
    let mut found = None;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (st, body) = http(
            &addr,
            "GET",
            "/admin/api/audit?event=kick",
            Some(&bearer),
            None,
        )
        .await;
        assert_eq!(st, 200);
        let events = body["events"].as_array().unwrap();
        if !events.is_empty() {
            found = Some(events[0].clone());
            break;
        }
    }
    let kick = found.expect("kick audit line");
    assert_eq!(kick["actor"], "admin", "console account attributed");
    assert_eq!(kick["target"], "bob");
    assert_eq!(kick["room"], "ik");
    assert_eq!(kick["reason"], "abuse of open mic");

    // 404s: unknown peer / unknown room
    let (st, _) = http(
        &addr,
        "DELETE",
        "/admin/api/rooms/ik/peers/ghost",
        Some(&bearer),
        None,
    )
    .await;
    assert_eq!(st, 404, "absent peer is 404, not silent success");
    let (st, _) = http(
        &addr,
        "DELETE",
        "/admin/api/rooms/noroof/peers/alice",
        Some(&bearer),
        None,
    )
    .await;
    assert_eq!(st, 404);

    // user tokens cannot touch the console at all
    let user_jwt = test_bridge::issue_token(USER_SECRET, "eav", "ik", Role::Field, 600);
    let (st, _) = http(
        &addr,
        "DELETE",
        "/admin/api/rooms/ik/peers/alice",
        Some(&user_jwt),
        None,
    )
    .await;
    assert_eq!(st, 401, "audience isolation holds on intervention routes");

    drop(a);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn force_release_shrinks_grants_with_rebroadcast() {
    let (addr, dir) = spawn_server().await;
    let bearer = admin_bearer(&addr).await;
    let mut a = join(&addr, "alice", "fr", Role::Field).await;

    a.send(WsMsg::Text(
        json!({"type": "floor_request", "priority": 0, "preempt": false})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    recv_type(&mut a, "floor_granted").await;

    // force-release via console (reason lands only in the audit line)
    let (st, resp) = http(
        &addr,
        "POST",
        "/admin/api/rooms/fr/peers/alice/force-release",
        Some(&bearer),
        Some(&json!({"reason": "recall"}).to_string()),
    )
    .await;
    assert_eq!(st, 204, "force-release: {resp}");

    // PIT-21 path: set shrank to empty -> FloorIdle reaches the still-connected holder
    recv_type(&mut a, "floor_idle").await;
    // socket stays open (release != kick): floor ops work again - after the
    // 500ms request cooldown clears (the re-request inside the window gets
    // floor_denied{rate_limited}; measured here the hard way)
    tokio::time::sleep(std::time::Duration::from_millis(650)).await;
    a.send(WsMsg::Text(
        json!({"type": "floor_request", "priority": 0, "preempt": false})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    recv_type(&mut a, "floor_granted").await;

    let mut found = None;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (st, body) = http(
            &addr,
            "GET",
            "/admin/api/audit?event=force_release",
            Some(&bearer),
            None,
        )
        .await;
        assert_eq!(st, 200);
        let events = body["events"].as_array().unwrap();
        if !events.is_empty() {
            found = Some(events[0].clone());
            break;
        }
    }
    let line = found.expect("force_release audit line");
    assert_eq!(line["actor"], "admin");
    assert_eq!(line["target"], "alice");
    assert_eq!(line["reason"], "recall");

    drop(a);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn operator_tier_may_intervene() {
    let (addr, dir) = spawn_server().await;
    let abearer = admin_bearer(&addr).await;
    // seed operator + join a room
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/accounts",
        Some(&abearer),
        Some(&json!({"user":"opx","password":"opx-password","role":"operator"}).to_string()),
    )
    .await;
    assert_eq!(st, 201);
    let (st, login) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(&json!({"user": "opx", "password": "opx-password"}).to_string()),
    )
    .await;
    assert_eq!(st, 200);
    let obearer = login["jwt"].as_str().unwrap().to_string();
    assert_eq!(login["role"], "operator");

    let _a = join(&addr, "alice", "op", Role::Field).await;
    let (st, resp) = http(
        &addr,
        "DELETE",
        "/admin/api/rooms/op/peers/alice",
        Some(&obearer),
        None,
    )
    .await;
    assert_eq!(
        st, 204,
        "operator intervention is a moderator duty, not admin-only: {resp}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
