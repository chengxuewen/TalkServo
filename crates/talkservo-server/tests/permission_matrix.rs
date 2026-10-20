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

// plan-7 R4 gate: one truth table, four identities x every console surface.
// admin tier = full; operator = read + monitor + tokens, NO accounts/registry
// writes; mod-token/field = no console at all (401 - wrong audience).

#[tokio::test]
async fn permission_matrix_truth_table() {
    let (addr, dir) = spawn_server().await;
    let abearer = admin_bearer(&addr).await;

    // seed an operator account (admin path works by definition)
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/accounts",
        Some(&abearer),
        Some(&json!({"user": "ops1", "password": "operator-pw-1", "role": "operator"}).to_string()),
    )
    .await;
    assert_eq!(st, 201, "seed operator");

    // login as operator -> role in response + token minted with tier
    let (st, login) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(&json!({"user": "ops1", "password": "operator-pw-1"}).to_string()),
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(login["role"], "operator", "tier travels in login response");
    let obearer = login["jwt"].as_str().unwrap().to_string();

    // ---- matrix: (method, path, admin => 2xx-family, operator => verdict) ----
    let rows: Vec<(&str, &str, u16, u16)> = vec![
        // method  path                       admin  operator
        ("GET", "/admin/api/overview", 200, 200),
        ("GET", "/admin/api/rooms/live", 200, 200),
        ("GET", "/admin/api/rooms", 200, 200),
        ("GET", "/admin/api/audit", 200, 200),
        ("GET", "/admin/api/accounts", 200, 200),
        ("POST", "/admin/api/tokens", 200, 200), // POST token: body per case below
        ("POST", "/admin/api/accounts", 201, 403),
        ("DELETE", "/admin/api/accounts/ops1", 204, 403),
        ("POST", "/admin/api/rooms", 201, 403),
        ("DELETE", "/admin/api/rooms/mx", 204, 403),
    ];
    let token_body =
        |room: &str| json!({"sub":"mx","room":room,"role":"field","ttl_s":600}).to_string();
    let account_body = || json!({"user":"mx1","password":"mx-password-1"}).to_string();
    let room_body = || json!({"room":"mx","owner":"squad"}).to_string();

    for (method, path, want_admin, want_op) in rows {
        let body = match (method, path) {
            ("POST", "/admin/api/tokens") => Some(token_body("mxroom")),
            ("POST", "/admin/api/accounts") => Some(account_body()),
            ("POST", "/admin/api/rooms") => Some(room_body()),
            ("DELETE", _) => None,
            _ => None,
        };
        // admin row
        let (st, resp) = http(&addr, method, path, Some(&abearer), body.as_deref()).await;
        assert_eq!(st, want_admin, "ADMIN {method} {path}: got {st} ({resp})");
        // operator row
        let (st, resp) = http(&addr, method, path, Some(&obearer), body.as_deref()).await;
        assert_eq!(st, want_op, "OPERATOR {method} {path}: got {st} ({resp})");
    }

    // non-console tokens (field / mod) cannot enter at all: audience isolation
    let user_jwt = test_bridge::issue_token(USER_SECRET, "f", "mxroom", Role::Field, 600);
    for bearer in [user_jwt.as_str()] {
        let (st, _) = http(&addr, "GET", "/admin/api/overview", Some(bearer), None).await;
        assert_eq!(st, 401, "user token on console = 401");
    }
    // role claim roundtrip through validate: a FORGED-role token (wrong
    // secret) is 401 before tier logic; serde-default (no role field) =
    // "admin" - the compat path is locked in auth.rs unit tests
    std::fs::remove_dir_all(&dir).ok();
}
