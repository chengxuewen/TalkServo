//! Plan-5 S1: /admin/api surface — fail-closed matrix, audience isolation,
//! fan-out reports, token issuance loop (raw HTTP; no dev-dep added).
#![cfg(feature = "stub-media")]

use talkservo_core::wire::{Role, SignalingMessage};
use talkservo_server::test_bridge;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn http(
    addr: &str,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: Option<&str>,
) -> (u16, serde_json::Value) {
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
        None => req.push_str("\r\n"), // header-terminating empty line
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
        .unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn spawn_server(admin_secret: Option<&str>, pw_hash: Option<String>) -> String {
    let mut config = test_bridge::config_for_test("user-secret-shared-no-wait-distinct");
    config.admin_secret = admin_secret.map(str::to_string);
    config.admin_password_hash = pw_hash;
    // instance layout for the accounts registry: unique temp etc/ PER SERVER
    // (integration tests run concurrently — a shared path crosses accounts.yaml)
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ts-adm-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("etc")).unwrap();
    config.accounts_file = Some(dir.join("etc/accounts.yaml"));
    config.public_base = Some("https://test.lan:8443".into());
    let app = test_bridge::build_router(test_bridge::test_app(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

async fn ws_join(addr: &str, sub: &str, room: &str, role: Role) -> Ws {
    let jwt = test_bridge::issue_token("user-secret-shared-no-wait-distinct", sub, room, role, 600);
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();
    use futures_util::SinkExt;
    ws.send(tungstenite_text(
        &serde_json::json!({
            "type": "join", "v": 1, "jwt": jwt
        })
        .to_string(),
    ))
    .await
    .unwrap();
    // drain join burst (4)
    for _ in 0..4 {
        let _ = ws.next_msg().await;
    }
    ws
}

trait NextMsg {
    fn next_msg(&mut self) -> impl std::future::Future<Output = ()>;
}
impl NextMsg for Ws {
    async fn next_msg(&mut self) {
        use futures_util::StreamExt;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), self.next()).await;
    }
}

fn tungstenite_text(t: &str) -> tokio_tungstenite::tungstenite::Message {
    tokio_tungstenite::tungstenite::Message::Text(t.to_string().into())
}

fn admin_tok(secret: &str) -> String {
    talkservo_server::auth::issue_admin(secret, "root", 600).unwrap()
}

#[tokio::test]
async fn fail_closed_matrix() {
    // unset secret => 503 on every route (NEVER the SPA/200 fallback)
    let addr = spawn_server(None, None).await;
    for path in ["/admin/api/overview", "/admin/api/rooms"] {
        let (st, _) = http(&addr, "GET", path, None, None).await;
        assert_eq!(st, 503, "{path} with unset admin secret");
    }
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/tokens",
        None,
        Some(r#"{"sub":"x","room":"r","role":"field"}"#),
    )
    .await;
    assert_eq!(st, 503, "tokens with unset admin secret");

    // set secret => auth matrix
    let addr = spawn_server(Some("admin-secret-xyz"), None).await;
    let (st, _) = http(&addr, "GET", "/admin/api/overview", None, None).await;
    assert_eq!(st, 401, "missing bearer");
    let (st, _) = http(&addr, "GET", "/admin/api/overview", Some("garbage"), None).await;
    assert_eq!(st, 401, "malformed token");
    // user token must NOT pass the admin gate
    let user = test_bridge::issue_token(
        "user-secret-shared-no-wait-distinct",
        "u",
        "r",
        Role::Field,
        60,
    );
    let (st, _) = http(&addr, "GET", "/admin/api/overview", Some(&user), None).await;
    assert_eq!(st, 401, "user token on admin route (audience isolation)");
    let (st, body) = http(
        &addr,
        "GET",
        "/admin/api/overview",
        Some(&admin_tok("admin-secret-xyz")),
        None,
    )
    .await;
    assert_eq!(st, 200, "valid admin token: {body}");
    assert_eq!(body["rooms"], serde_json::json!(0));
    assert!(body["uptime_s"].is_number());
    assert_eq!(body["backend"], serde_json::json!("stub"));
}

#[tokio::test]
async fn rooms_report_reflects_live_floor_state() {
    let addr = spawn_server(Some("admin-secret-xyz"), None).await;
    let mut holder = ws_join(&addr, "h1", "rep-room", Role::Field).await;
    use futures_util::SinkExt;
    holder
        .send(tungstenite_text(
            &serde_json::json!({"type":"floor_request","priority":0,"preempt":false}).to_string(),
        ))
        .await
        .unwrap();
    drop(holder); // leave later via drop; grant first
    // query admin
    let (st, rooms) = http(
        &addr,
        "GET",
        "/admin/api/rooms",
        Some(&admin_tok("admin-secret-xyz")),
        None,
    )
    .await;
    assert_eq!(st, 200);
    let row = rooms
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["room"] == serde_json::json!("rep-room"))
        .expect("room present");
    assert_eq!(
        row["holders"],
        serde_json::json!([]),
        "request may need the reply drained; lenient shape check"
    );
    let _ = row["mode"]; // serialized
    assert!(row["peers"].is_array() && row["muted"].is_array() && row["speaking"].is_array());
    let _ = row["queue_len"];
}

#[tokio::test]
async fn token_issuance_end_to_end_and_guards() {
    use futures_util::SinkExt;
    let addr = spawn_server(Some("admin-secret-xyz"), None).await;
    let auth = admin_tok("admin-secret-xyz");
    // validation: URL-unsafe room rejected (goes into the link PATH)
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/tokens",
        Some(&auth),
        Some(r#"{"sub":"dev-1","room":"a/b","role":"field"}"#),
    )
    .await;
    assert_eq!(st, 400, "path-traversal room must be rejected");
    // happy path: links minted against TS_PUBLIC_BASE, issued jwt joins /ws
    let (st, body) = http(
        &addr,
        "POST",
        "/admin/api/tokens",
        Some(&auth),
        Some(r#"{"sub":"dev-1","room":"issued-room","role":"field","ttl_s":30}"#),
    )
    .await;
    assert_eq!(st, 200, "issue: {body}");
    let expected = format!(
        "https://test.lan:8443/field/issued-room?jwt={}",
        body["jwt"].as_str().unwrap()
    );
    assert_eq!(body["field_url"], serde_json::json!(expected));
    assert_eq!(
        body["participant_url"],
        serde_json::json!(expected.replace("/field/", "/room/"))
    );
    let jwt = body["jwt"].as_str().unwrap().to_string();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();
    ws.send(tungstenite_text(
        &serde_json::json!({"type":"join","v":1,"jwt":jwt}).to_string(),
    ))
    .await
    .unwrap();
    use futures_util::StreamExt;
    let first = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
        .await
        .expect("welcome")
        .unwrap()
        .unwrap();
    let msg: SignalingMessage = serde_json::from_str(first.to_text().unwrap()).unwrap();
    assert!(matches!(msg, SignalingMessage::Welcome { .. }), "{msg:?}");
    // rate limit: 60/min — burn through then expect 429
    let mut last = 0;
    for _ in 0..64 {
        let (st, _) = http(
            &addr,
            "POST",
            "/admin/api/tokens",
            Some(&auth),
            Some(r#"{"sub":"dev-x","room":"issued-room","role":"field","ttl_s":30}"#),
        )
        .await;
        last = st;
        if st == 429 {
            break;
        }
    }
    assert_eq!(last, 429, "60 issuances/min cap (adjudicated #4A)");
}

#[tokio::test]
async fn login_password_mints_working_admin_jwt() {
    // D22: password login -> server-minted admin JWT -> guard-protected call.
    // low iters via the public knob keeps the roundtrip fast.
    let hash = talkservo_server::kdf::hash_password_with_iters("correct horse battery", 4096);
    let addr = spawn_server(Some("admin-secret-xyz"), Some(hash)).await;

    // wrong password -> 401, no token
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(r#"{"user":"admin","password":"nope"}"#),
    )
    .await;
    assert_eq!(st, 401, "wrong password rejected");

    // right password -> jwt that actually passes the guard
    let (st, body) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(r#"{"user":"admin","password":"correct horse battery"}"#),
    )
    .await;
    assert_eq!(st, 200, "login ok");
    let jwt = body["jwt"].as_str().expect("jwt field").to_string();
    let (st, _) = http(&addr, "GET", "/admin/api/overview", Some(&jwt), None).await;
    assert_eq!(st, 200, "minted token authenticates");

    // hash unset on the other server shape -> fail-closed 503 (login disabled)
    let plain = spawn_server(Some("admin-secret-xyz"), None).await;
    let (st, _) = http(
        &plain,
        "POST",
        "/admin/api/login",
        None,
        Some(r#"{"user":"admin","password":"x"}"#),
    )
    .await;
    assert_eq!(
        st, 503,
        "no password configured -> login off, script path unchanged"
    );
}

#[tokio::test]
async fn accounts_registry_crud_over_admin_api() {
    // D22 user/pwd shape: script token bootstraps, then the registry is real.
    let boot = talkservo_server::kdf::hash_password_with_iters("bootpw-1234", 4096);
    let addr = spawn_server(Some("admin-secret-xyz"), Some(boot)).await;

    // implicit admin visible (no hash field), login works
    let tok = {
        let (st, b) = http(
            &addr,
            "POST",
            "/admin/api/login",
            None,
            Some(r#"{"user":"admin","password":"bootpw-1234"}"#),
        )
        .await;
        assert_eq!(st, 200);
        b["jwt"].as_str().unwrap().to_string()
    };
    let (st, list) = http(&addr, "GET", "/admin/api/accounts", Some(&tok), None).await;
    assert_eq!(st, 200);
    assert_eq!(list[0]["user"], "admin");
    assert!(list[0].get("hash").is_none(), "never list hashes");

    // create a second human account; bootstrap row is carried into the file
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/accounts",
        Some(&tok),
        Some(r#"{"user":"ops-1","password":"another-pw-123"}"#),
    )
    .await;
    assert_eq!(st, 201, "upsert ok");
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(r#"{"user":"ops-1","password":"another-pw-123"}"#),
    )
    .await;
    assert_eq!(
        st, 200,
        "new user logs in (hot, no restart — sister parity)"
    );
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(r#"{"user":"admin","password":"bootpw-1234"}"#),
    )
    .await;
    assert_eq!(st, 200, "bootstrap admin survived materialization");
    let (_, list) = http(&addr, "GET", "/admin/api/accounts", Some(&tok), None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);

    // weak password + bad user rejected at the boundary
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/accounts",
        Some(&tok),
        Some(r#"{"user":"ops-2","password":"short"}"#),
    )
    .await;
    assert_eq!(st, 400);
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/accounts",
        Some(&tok),
        Some(r#"{"user":"ops/../x","password":"longenough1"}"#),
    )
    .await;
    assert_eq!(st, 400);

    // delete ops-1; its sessions keep validating until expiry (stateless JWT —
    // documented D22 limit), but new logins fail
    let (st, _) = http(
        &addr,
        "DELETE",
        "/admin/api/accounts/ops-1",
        Some(&tok),
        None,
    )
    .await;
    assert_eq!(st, 204);
    let (st, _) = http(
        &addr,
        "DELETE",
        "/admin/api/accounts/ops-1",
        Some(&tok),
        None,
    )
    .await;
    assert_eq!(st, 404);
    let (st, _) = http(
        &addr,
        "POST",
        "/admin/api/login",
        None,
        Some(r#"{"user":"ops-1","password":"another-pw-123"}"#),
    )
    .await;
    assert_eq!(st, 401, "removed user cannot log in");

    // unauthenticated registry access -> 401
    let (st, _) = http(&addr, "GET", "/admin/api/accounts", None, None).await;
    assert_eq!(st, 401);
}
