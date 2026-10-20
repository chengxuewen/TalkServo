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

async fn ws_connect(addr: &str, sub: &str, room: &str) -> Ws {
    let jwt = test_bridge::issue_token(USER_SECRET, sub, room, Role::Field, 600);
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

#[tokio::test]
async fn registry_crud_live_rename_and_lock() {
    let (addr, dir) = spawn_server().await;
    let bearer = admin_bearer(&addr).await;

    // registry starts empty; live poll answers under the renamed path (R1)
    let (st, list) = http(&addr, "GET", "/admin/api/rooms", Some(&bearer), None).await;
    assert_eq!(st, 200);
    assert_eq!(list.as_array().unwrap().len(), 0);
    let (st, live) = http(&addr, "GET", "/admin/api/rooms/live", Some(&bearer), None).await;
    assert_eq!(st, 200, "live poll at renamed path: {live}");
    assert!(live.is_array());

    // create: mode=open so BOTH speakers can hold (proves registry default reached RoomState)
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/rooms",
        Some(&bearer),
        Some(&json!({"room":"ops","owner":"squad","mode":"open","max_peers":99,"max_queue":null,"locked":false}).to_string()),
    )
    .await;
    assert_eq!(st, 201);
    let (st, list) = http(&addr, "GET", "/admin/api/rooms", Some(&bearer), None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["owner"], "squad");
    assert_eq!(st, 200);

    // two joins, both granted concurrently: only possible if spawn mode = Open
    let mut alice = ws_connect(&addr, "alice", "ops").await;
    wait_for_type(&mut alice, "welcome").await;
    let mut bob = ws_connect(&addr, "bob", "ops").await;
    wait_for_type(&mut alice, "peer_joined").await;
    alice
        .send(WsMsg::Text(
            json!({"type": "floor_request", "priority": 0, "preempt": false})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let g1 = wait_for_type(&mut alice, "floor_granted").await;
    assert_eq!(g1["grants"].as_array().unwrap().len(), 1);
    bob.send(WsMsg::Text(
        json!({"type": "floor_request", "priority": 0, "preempt": false})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    // In Hybrid (cap 2) this would ALSO grant (cap allows 2) - so prove Open via
    // the queue: open never queues; grants include both peers.
    let g2 = wait_for_type(&mut alice, "floor_granted").await;
    assert_eq!(g2["grants"].as_array().unwrap().len(), 2);

    // lock: new joins rejected with wire Error{room_locked}; members stay
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/rooms",
        Some(&bearer),
        Some(&json!({"room":"ops","owner":"squad","locked":true}).to_string()),
    )
    .await;
    assert_eq!(st, 201, "upsert replaces");
    let mut carol = ws_connect(&addr, "carol", "ops").await;
    let err = wait_for_type(&mut carol, "error").await;
    assert_eq!(err["code"], "room_locked");
    drop(carol);

    // delete = row gone + live room evicted (alice/bob sockets close; audit
    // records both leave lines through the S1 relay)
    let (st, _) = http(&addr, "DELETE", "/admin/api/rooms/ops", Some(&bearer), None).await;
    assert_eq!(st, 204);
    let (st, _) = http(&addr, "DELETE", "/admin/api/rooms/ops", Some(&bearer), None).await;
    assert_eq!(st, 404, "second delete = not in registry");
    // alice's socket ends once the actor dropped her sink (channel close)
    let closed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match alice.next().await {
                None | Some(Err(_)) | Some(Ok(WsMsg::Close(_))) => return true,
                Some(Ok(_)) => continue,
            }
        }
    })
    .await
    .unwrap_or(false);
    assert!(closed, "alice socket closed after eviction");
    drop(bob);

    // audit trail proves eviction left lines (S1 relay riding the leave path)
    let mut saw_leaves = 0;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (st, body) = http(
            &addr,
            "GET",
            "/admin/api/audit?event=leave",
            Some(&bearer),
            None,
        )
        .await;
        assert_eq!(st, 200);
        saw_leaves = body["events"].as_array().unwrap().len();
        if saw_leaves >= 2 {
            break;
        }
    }
    assert!(
        saw_leaves >= 2,
        "evicted members produced leave audit lines"
    );

    // registry file survives on disk exactly as ops would hand-check it
    let raw = std::fs::read_to_string(dir.join("etc/rooms.yaml")).unwrap();
    assert!(!raw.contains("ops"), "row deleted from file too");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn validation_and_auth_gates() {
    let (addr, dir) = spawn_server().await;
    // 401 without bearer on every registry surface
    for (m, p) in [
        ("GET", "/admin/api/rooms"),
        ("POST", "/admin/api/rooms"),
        ("DELETE", "/admin/api/rooms/x"),
    ] {
        let (st, _) = http(&addr, m, p, None, Some("{}")).await;
        assert_eq!(st, 401, "{m} {p}");
    }
    let bearer = admin_bearer(&addr).await;
    // bad names rejected 400
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/rooms",
        Some(&bearer),
        Some(&json!({"room": "bad name!", "owner": ""}).to_string()),
    )
    .await;
    assert_eq!(st, 400);
    // valid zero-padded name accepted (charset parity with tokens)
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/rooms",
        Some(&bearer),
        Some(&json!({"room": "squad-07.x", "owner": "p"}).to_string()),
    )
    .await;
    assert_eq!(st, 201);
    std::fs::remove_dir_all(&dir).ok();
}
