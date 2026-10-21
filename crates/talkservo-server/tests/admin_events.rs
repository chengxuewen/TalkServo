#![cfg(feature = "stub-media")] // test_bridge is stub-gated (ws_flow convention)
//! Plan-9 W1: the admin push channel (`/admin/ws`).
//!
//! Same bridge as `admin_api.rs` (`build_router` over `test_app`), so the budget,
//! the close codes and the fan-out are exercised on the wiring the binary actually
//! runs — not on a hand-stacked router that could drift from it.

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use talkservo_server::admin_events::ADMIN_WS_CONNECTS_PER_MIN;
use talkservo_server::test_bridge;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMsg;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

const ADMIN_SECRET: &str = "plan9-ws-secret";

fn text(t: &str) -> WsMsg {
    WsMsg::Text(t.into())
}

fn admin_tok() -> String {
    talkservo_server::auth::issue_admin(ADMIN_SECRET, "root", 600).unwrap()
}

/// (addr, audit path). `audit` decides whether the writer — and with it the hub's
/// only producer — exists at all, which is the coupling this file pins.
async fn spawn_server(audit: bool) -> (String, std::path::PathBuf) {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ts-admin-ws-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("etc")).unwrap();
    let audit_path = dir.join("logs/audit.jsonl");
    let mut config = test_bridge::config_for_test("user-secret-plan9");
    config.admin_secret = Some(ADMIN_SECRET.to_string());
    config.accounts_file = Some(dir.join("etc/accounts.yaml"));
    config.audit_log = audit.then_some(audit_path.clone());
    let app = test_bridge::build_router(test_bridge::test_app(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, audit_path)
}

async fn open(addr: &str) -> Ws {
    connect_async(format!("ws://{addr}/admin/ws"))
        .await
        .expect("upgrade")
        .0
}

/// Next text frame as JSON, or None on close/timeout.
async fn next_json(ws: &mut Ws) -> Option<Value> {
    for _ in 0..8 {
        let Ok(Some(m)) = tokio::time::timeout(std::time::Duration::from_secs(3), ws.next()).await
        else {
            return None;
        };
        match m.ok()? {
            WsMsg::Text(t) => return serde_json::from_str(&t).ok(),
            WsMsg::Close(_) => return None,
            _ => continue,
        }
    }
    None
}

/// Assert a close happens (and quickly) without any data frame first.
async fn expect_closed(ws: &mut Ws) {
    let Ok(Some(m)) = tokio::time::timeout(std::time::Duration::from_secs(3), ws.next()).await
    else {
        panic!("no close frame received");
    };
    match m {
        Ok(WsMsg::Close(_)) | Err(_) => {}
        Ok(other) => panic!("expected a close, got a data frame: {other:?}"),
    }
}

async fn issue_token_http(addr: &str, sub: &str) {
    let body =
        format!("{{\"sub\":\"{sub}\",\"room\":\"ws-room\",\"role\":\"field\",\"ttl_s\":600}}");
    let req = format!(
        "POST /admin/api/tokens HTTP/1.1\r\nhost: t\r\nauthorization: Bearer {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        admin_tok(),
        body.len()
    );
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        stream.read_to_end(&mut buf),
    )
    .await
    .expect("read guard")
    .expect("read");
    let head = String::from_utf8_lossy(&buf).into_owned();
    assert!(
        head.starts_with("HTTP/1.1 201") || head.starts_with("HTTP/1.1 200"),
        "token issuance should succeed, got: {}",
        head.lines().next().unwrap_or("")
    );
}

fn read_lines(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn the_typescript_mirror_cannot_drift_from_the_rust_enums() {
    // The signaling wire has a replay-fixture guard (ws-transcript.json), which by
    // construction only covers frames a test actually SENT. This covers the other
    // half — the DECLARED variant set — so adding an AdminEvent in Rust and
    // forgetting web/src/lib/adminEvents.ts (or the reverse) fails here without
    // anyone having to think of sending it. Kept in the Rust lane on purpose: the
    // web tsconfig has no node types, and this comparison needs a filesystem.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let rust = std::fs::read_to_string(manifest.join("src/admin_events.rs")).unwrap();
    let ts = std::fs::read_to_string(manifest.join("../../web/src/lib/adminEvents.ts"))
        .expect("the console-side mirror must exist");

    let snake = |name: &str| {
        let mut out = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_ascii_uppercase() && i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        }
        out
    };
    // variants of a Rust enum: `    Name {` or `    Name,` inside its body
    let rust_variants = |enum_name: &str| -> Vec<String> {
        let start = rust
            .find(&format!("pub enum {enum_name} {{"))
            .unwrap_or_else(|| panic!("{enum_name} missing from admin_events.rs"));
        // str::find takes no offset, so slice first and search the remainder
        let rest = &rust[start..];
        let body = &rest[..rest.find("\n}").expect("enum close")];
        let mut v = Vec::new();
        for line in body.lines().skip(1) {
            let t = line.trim_end();
            if let Some(rest) = t.strip_prefix("    ") {
                let head = rest.trim_end_matches(" {").trim_end_matches(',');
                let ident: String = head
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if ident.starts_with(|c: char| c.is_ascii_uppercase()) && !ident.starts_with("///")
                {
                    v.push(snake(&ident));
                }
            }
        }
        v
    };
    // `type: "x"` literals of a TS discriminated union, bounded by the next export
    let ts_tags = |type_name: &str| -> Vec<String> {
        let start = ts
            .find(&format!("export type {type_name} ="))
            .unwrap_or_else(|| panic!("{type_name} missing from adminEvents.ts"));
        let rest = &ts[start..];
        let end = rest.find("\nexport ").unwrap_or(rest.len());
        let mut v = Vec::new();
        let mut cursor = 0;
        while let Some(i) = rest[cursor..end].find("type: \"") {
            let from = cursor + i + 7;
            let to = from + rest[from..].find('"').unwrap();
            v.push(rest[from..to].to_string());
            cursor = to;
        }
        v
    };

    for (enum_name, type_name) in [("AdminEvent", "AdminEvent"), ("AdminIn", "AdminIn")] {
        let mut r = rust_variants(enum_name);
        let mut t = ts_tags(type_name);
        r.sort();
        t.sort();
        assert!(!r.is_empty(), "no variants parsed from {enum_name}");
        assert_eq!(r, t, "{type_name}: TS mirror drifted from the Rust enum");
    }
    assert_eq!(
        rust.matches("rename_all = \"snake_case\"").count(),
        2,
        "both admin enums must stay tagged snake_case enums (modules/02 discipline)"
    );
    assert!(
        ts.contains("export const ADMIN_EVENT_V = 1;"),
        "version mirrored"
    );
}

#[tokio::test]
async fn anything_but_an_auth_frame_closes_the_socket() {
    let (addr, _) = spawn_server(true).await;
    let mut ws = open(&addr).await;
    ws.send(text(r#"{"type":"ping"}"#)).await.unwrap();
    expect_closed(&mut ws).await;
}

#[tokio::test]
async fn a_user_token_cannot_subscribe_the_admin_plane() {
    // The audience isolation has a second direction now that there is an admin
    // socket: a USER token (aud = room) must be refused HERE, just as an admin
    // token is refused by /ws.
    let (addr, _) = spawn_server(true).await;
    let mut ws = open(&addr).await;
    let user_jwt = test_bridge::issue_token(
        "user-secret-plan9",
        "not-admin",
        "ws-room",
        talkservo_core::wire::Role::Field,
        600,
    );
    ws.send(text(&format!(r#"{{"type":"auth","token":"{user_jwt}"}}"#)))
        .await
        .unwrap();
    expect_closed(&mut ws).await;
}

#[tokio::test]
async fn hello_then_hint_then_live_on_a_real_mutation() {
    let (addr, audit) = spawn_server(true).await;
    let mut ws = open(&addr).await;
    ws.send(text(&format!(
        r#"{{"type":"auth","token":"{}"}}"#,
        admin_tok()
    )))
    .await
    .unwrap();

    let hello = next_json(&mut ws).await.expect("hello");
    assert_eq!(hello["type"], "hello");
    assert_eq!(hello["v"], 1);
    assert_eq!(hello["dropped"], 0, "fresh server has evicted nobody");
    assert_eq!(
        hello["debounce_ms"],
        u64::try_from(talkservo_server::admin_events::ROOMS_DEBOUNCE.as_millis()).unwrap(),
        "the socket must be told the window the server actually runs, not a copy"
    );

    issue_token_http(&addr, "probe-1").await;

    let hint = next_json(&mut ws).await.expect("hint");
    assert_eq!(hint["type"], "audit_hint");
    assert!(
        hint.get("seq").is_none(),
        "a hint must not carry ledger content (plan-9 R3=B): {hint}"
    );

    let live = next_json(&mut ws).await.expect("rooms snapshot");
    assert_eq!(live["type"], "rooms_live");
    assert!(live["rooms"].is_array(), "rooms payload expected: {live}");

    // and the same action did land in the file — hint and ledger agree because
    // they share one producer, not because the socket copied the row
    let lines = read_lines(&audit);
    assert!(
        lines.iter().any(|l| l.contains("token_issue")),
        "audit file should carry the line the hint announced: {lines:?}"
    );
    let token = admin_tok();
    assert!(
        !lines.iter().any(|l| l.contains(&token)),
        "tokens never reach the audit trail (D22 rule)"
    );
}

#[tokio::test]
async fn no_audit_writer_means_no_push_and_still_a_working_socket() {
    // Documented coupling (plan-9 §3.3): the audit emit is the hub's only
    // producer. With audit off there is nothing to announce — which must degrade
    // to "the console reconciles", never to "the console lies".
    let (addr, _) = spawn_server(false).await;
    let mut ws = open(&addr).await;
    ws.send(text(&format!(
        r#"{{"type":"auth","token":"{}"}}"#,
        admin_tok()
    )))
    .await
    .unwrap();
    assert_eq!(
        next_json(&mut ws).await.expect("hello")["type"],
        "hello",
        "the socket authenticates regardless of the audit writer"
    );
    issue_token_http(&addr, "probe-2").await;
    assert!(
        next_json(&mut ws).await.is_none(),
        "no writer => no hints: RoomsLive would be silence, not a lie"
    );
}

#[tokio::test]
async fn the_connect_budget_is_its_own_window() {
    // R5=A: /admin/ws gets its own counter so a wedged page looping
    // `new WebSocket()` can never consume the login/token budget. Attempts are
    // counted before any credential exists, because that is what a storm is.
    let (addr, _) = spawn_server(true).await;
    for i in 0..ADMIN_WS_CONNECTS_PER_MIN {
        let ws = connect_async(format!("ws://{addr}/admin/ws")).await;
        assert!(
            ws.is_ok(),
            "connect {i} of {ADMIN_WS_CONNECTS_PER_MIN} should still be allowed: {:?}",
            ws.err()
        );
        drop(ws.unwrap().0);
    }
    let blocked = connect_async(format!("ws://{addr}/admin/ws")).await;
    let err = blocked.expect_err("budget should have been reached");
    let status = match &err {
        tokio_tungstenite::tungstenite::Error::Http(resp) => Some(resp.status().as_u16()),
        _ => None,
    };
    assert_eq!(
        status,
        Some(429),
        "expected 429 beyond the budget, got {err:?}"
    );
}

#[tokio::test]
async fn two_tabs_both_receive_the_same_hint() {
    // one producer, N sinks: the coalesced fan-out is shared, not per-tab
    let (addr, _) = spawn_server(true).await;
    let mut a = open(&addr).await;
    let mut b = open(&addr).await;
    for ws in [&mut a, &mut b] {
        ws.send(text(&format!(
            r#"{{"type":"auth","token":"{}"}}"#,
            admin_tok()
        )))
        .await
        .unwrap();
        assert_eq!(next_json(ws).await.expect("hello")["type"], "hello");
    }
    issue_token_http(&addr, "probe-3").await;
    assert_eq!(
        next_json(&mut a).await.expect("hint a")["type"],
        "audit_hint"
    );
    assert_eq!(
        next_json(&mut b).await.expect("hint b")["type"],
        "audit_hint"
    );
}
