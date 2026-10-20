#![cfg(feature = "stub-media")] // test_bridge is stub-gated (ws_flow convention)
// plan-7 S1: the audit trail records floor + join/leave + admin events, the
// reader pages by seq cursor with filters, and the endpoint sits behind the
// admin guard (401/503 matrix). Stub lane; writer task flushed via polling.
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use talkservo_core::wire::Role;
use talkservo_server::test_bridge;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message as WsMsg;

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const USER_SECRET: &str = "audit-flow-user-secret";
const ADMIN_SECRET: &str = "audit-flow-admin-secret";
const PASSWORD: &str = "audit-flow-pw";

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
        Some(b) => {
            req.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{b}",
                b.len()
            ));
        }
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
    .expect("http response (hang guard)")
    .expect("read");
    let text = String::from_utf8_lossy(&raw).to_string();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status line");
    let json = text
        .split("\r\n\r\n")
        .nth(1)
        .and_then(|b| serde_json::from_str(b).ok())
        .unwrap_or(Value::Null);
    (status, json)
}

async fn spawn_server(audit: bool) -> (String, std::path::PathBuf) {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ts-audit-flow-{}-{}",
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
    config.public_base = Some("https://test.lan:8443".into());
    let afile = dir.join("logs/audit.jsonl");
    if audit {
        config.audit_log = Some(afile.clone());
    }
    let app = test_bridge::build_router(test_bridge::test_app(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, afile)
}

async fn ws_join(addr: &str, sub: &str, room: &str, role: Role) -> Ws {
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
    // drain through welcome + peer list
    recv_until(&mut ws, |m| m["type"] == "welcome").await;
    ws
}

async fn recv_until<F: Fn(&Value) -> bool>(ws: &mut Ws, stop: F) -> Value {
    for _ in 0..24 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
            .await
            .expect("frame timeout")
            .expect("stream open")
            .expect("text frame");
        let v: Value = serde_json::from_str(frame.to_text().unwrap()).unwrap();
        if stop(&v) {
            return v;
        }
    }
    panic!("stop condition never matched");
}

async fn poll_audit(addr: &str, bearer: &str, want: &[&str]) -> Vec<Value> {
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (st, body) = http(
            addr,
            "GET",
            "/admin/api/audit?limit=100",
            Some(bearer),
            None,
        )
        .await;
        assert_eq!(st, 200, "audit query: {body}");
        let events = body["events"].as_array().cloned().unwrap_or_default();
        let kinds: Vec<&str> = events.iter().filter_map(|e| e["event"].as_str()).collect();
        if want.iter().all(|w| kinds.contains(w)) {
            return events;
        }
    }
    panic!("audit trail never showed all of {want:?}");
}

#[tokio::test]
async fn trail_records_floor_join_and_admin_events() {
    let (addr, _afile) = spawn_server(true).await;
    let mut alice = ws_join(&addr, "alice", "aroom", Role::Field).await;
    let chief = ws_join(&addr, "chief", "aroom", Role::Dispatch).await;

    alice
        .send(WsMsg::Text(
            json!({"type": "floor_request", "priority": 0, "preempt": false})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    recv_until(&mut alice, |m| m["type"] == "floor_granted").await;
    alice
        .send(WsMsg::Text(
            json!({"type": "floor_release"}).to_string().into(),
        ))
        .await
        .unwrap();
    recv_until(&mut alice, |m| m["type"] == "floor_idle").await;

    let (st, login) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(&json!({"user": "admin", "password": PASSWORD}).to_string()),
    )
    .await;
    assert_eq!(st, 200, "login: {login}");
    let bearer = login["jwt"].as_str().unwrap().to_string();
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(&json!({"user": "admin", "password": "wrong"}).to_string()),
    )
    .await;
    assert_eq!(st, 401);
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/tokens",
        Some(&bearer),
        Some(&json!({"sub": "dev-1", "room": "r2", "role": "field", "ttl_s": 30}).to_string()),
    )
    .await;
    assert_eq!(st, 200);

    drop(chief); // leave event via socket close
    let events = poll_audit(
        &addr,
        &bearer,
        &[
            "join",
            "floor_grant",
            "floor_idle",
            "login_fail",
            "token_issue",
            "leave",
        ],
    )
    .await;

    // newest-first + seq cursor
    let seqs: Vec<u64> = events.iter().filter_map(|e| e["seq"].as_u64()).collect();
    assert!(
        seqs.windows(2).all(|w| w[0] > w[1]),
        "newest first: {seqs:?}"
    );
    let before = seqs[seqs.len() - 1];
    let (st, body) = http(
        &addr,
        "GET",
        &format!("/admin/api/audit?before={before}"),
        Some(&bearer),
        None,
    )
    .await;
    assert_eq!(st, 200);
    for e in body["events"].as_array().unwrap() {
        assert!(e["seq"].as_u64().unwrap() < before, "cursor before");
    }
    // actor filter: grant line carries alice
    let (st, body) = http(
        &addr,
        "GET",
        "/admin/api/audit?event=floor_grant&actor=alice",
        Some(&bearer),
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert!(
        !body["events"].as_array().unwrap().is_empty(),
        "grant attributed to alice"
    );
    // room attribution survived the relay
    let grant = &body["events"].as_array().unwrap()[0];
    assert_eq!(grant["room"], "aroom");
    assert_eq!(grant["changes"][0]["new"], json!(["alice"]));
    // dropped counter is part of the contract (R4 fail-visible)
    assert!(body["dropped"].is_number());

    // endpoint is auth-guarded
    let (st, _) = http(&addr, "GET", "/admin/api/audit", None, None).await;
    assert_eq!(st, 401, "audit requires admin bearer");
}

#[tokio::test]
async fn audit_endpoint_503_when_disabled() {
    let (addr, _) = spawn_server(false).await;
    let (st, login) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(&json!({"user": "admin", "password": PASSWORD}).to_string()),
    )
    .await;
    assert_eq!(st, 200);
    let bearer = login["jwt"].as_str().unwrap().to_string();
    let (st, _) = http(&addr, "GET", "/admin/api/audit", Some(&bearer), None).await;
    assert_eq!(st, 503, "audit disabled reads fail closed");
}
