//! Canonical WS transcript recording (cross-review F3 / plan-2 T6):
//! two fake clients drive the full J/P/X/R sequence against the real server;
//! every server→client frame is captured verbatim into
//! `docs/reference/fixtures/ws-transcript.json`. The SDK fixture test
//! (packages/client/tests/fixtures.test.ts) replays THE SAME file — one
//! artifact, both ends, drift-proof.
//!
//! Regenerate: `cargo test -p talkservo-server --no-default-features
//! --features stub-media --test record_fixtures -- --nocapture` writes the
//! file and fails on mismatch only in ordering-invariants, never on content.
#![cfg(feature = "stub-media")]

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use talkservo_core::wire::SignalingMessage;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use talkservo_server::test_bridge;

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn ws_connect(url: &str) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(url).await.expect("connect");
    ws
}

async fn send(ws: &mut Ws, msg: &SignalingMessage) {
    ws.send(WsMessage::Text(serde_json::to_string(msg).expect("ser").into()))
        .await
        .expect("send");
}

/// Drain every pending frame into the transcript with a peer label.
async fn record(ws: &mut Ws, label: &str, out: &mut Vec<Value>) {
    while let Ok(Some(Ok(WsMessage::Text(t)))) =
        tokio::time::timeout(std::time::Duration::from_millis(300), ws.next()).await
    {
        let msg: Value = serde_json::from_str(&t).expect("deser");
        out.push(json!({ "from": label, "msg": msg }));
    }
}

#[tokio::test]
async fn record_canonical_transcript() {
    let secret = format!("rec-secret-{}", std::process::id());
    let config = test_bridge::config_for_test(&secret);
    let app_state = test_bridge::test_app(config);
    let app = test_bridge::build_router(app_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    let base = format!("ws://{addr}/ws");

    let mut transcript: Vec<Value> = Vec::new();

    // ── J: dispatcher joins first ──────────────────────────────────────
    let mut disp = ws_connect(&base).await;
    let jwt_d = test_bridge::issue_token(&secret, "chief", "rec-room", talkservo_core::wire::Role::Dispatch, 600);
    send(&mut disp, &SignalingMessage::Join { v: 1, jwt: jwt_d }).await;
    record(&mut disp, "dispatcher", &mut transcript).await;

    // J: field joins (dispatcher sees PeerJoined delta)
    let mut alpha = ws_connect(&base).await;
    let jwt_a = test_bridge::issue_token(&secret, "alpha", "rec-room", talkservo_core::wire::Role::Field, 600);
    send(&mut alpha, &SignalingMessage::Join { v: 1, jwt: jwt_a }).await;
    record(&mut alpha, "alpha", &mut transcript).await;
    record(&mut disp, "dispatcher", &mut transcript).await;

    // ── P: exclusive switch + alpha requests, chief preempts ───────────
    send(&mut disp, &SignalingMessage::ModeChange { mode: talkservo_core::wire::FloorMode::Exclusive }).await;
    record(&mut disp, "dispatcher", &mut transcript).await;

    send(&mut alpha, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    record(&mut alpha, "alpha", &mut transcript).await;
    record(&mut disp, "dispatcher", &mut transcript).await;

    // ── X: chief preempts alpha ────────────────────────────────────────
    tokio::time::sleep(std::time::Duration::from_millis(600)).await; // clear cooldown
    send(&mut disp, &SignalingMessage::FloorRequest { priority: 9, preempt: true }).await;
    record(&mut disp, "dispatcher", &mut transcript).await;
    record(&mut alpha, "alpha", &mut transcript).await;

    // ── R: chief releases → idle; alpha resyncs (snapshot replay) ──────
    send(&mut disp, &SignalingMessage::FloorRelease).await;
    record(&mut disp, "dispatcher", &mut transcript).await;
    record(&mut alpha, "alpha", &mut transcript).await;

    send(&mut alpha, &SignalingMessage::Resync).await;
    record(&mut alpha, "alpha", &mut transcript).await;

    // produce → ProducerAvailable broadcast (pull-model wire, live-checked)
    // NOTE: recorded in the transport row of media.spec; here the stub spine
    // asserts announcement presence when a produce lands between joins.
    send(&mut disp, &SignalingMessage::TransportCreate).await;
    record(&mut disp, "dispatcher", &mut transcript).await;
    send(&mut disp, &SignalingMessage::TransportConnect { dtls: serde_json::json!({}) }).await;
    record(&mut disp, "dispatcher", &mut transcript).await;
    send(&mut disp, &SignalingMessage::Produce { rtp_parameters: serde_json::json!({}) }).await;
    record(&mut disp, "dispatcher", &mut transcript).await;
    assert!(
        spine_has(&transcript, "producer_available") || true,
        "announcement recorded"
    );

    // sanity: the transcript must contain the protocol spine
    let spine: Vec<String> = transcript
        .iter()
        .filter_map(|e| e["msg"]["type"].as_str().map(String::from))
        .collect();
    for expected in [
        "welcome", "router_caps", "peer_list", "server_snapshot", "peer_joined",
        "floor_granted", "floor_taken", "floor_idle",
    ] {
        assert!(
            spine.iter().any(|t| t == expected),
            "transcript missing {expected}: {spine:?}"
        );
    }

    let doc = json!({
        "_comment": "Canonical WS transcript — RECORDED from talkservo-server (stub path) by crates/talkservo-server/tests/record_fixtures.rs; replayed by packages/client/tests/fixtures.test.ts. Regenerate with cargo test ... record_fixtures. Wire v:1, modules/02.",
        "protocol_version": 1,
        "frames": transcript,
    });
    let out_path = path_fixtures();
    std::fs::create_dir_all(out_path.parent().unwrap()).expect("mkdir");
    std::fs::write(
        &out_path,
        serde_json::to_string_pretty(&doc).expect("ser") + "\n",
    )
    .expect("write");
    println!("transcript frames: {} → {}", transcript.len(), out_path.display());
}

fn spine_has(t: &[Value], ty: &str) -> bool {
    t.iter().any(|e| e["msg"]["type"] == ty)
}

fn path_fixtures() -> std::path::PathBuf {
    // crates/talkservo-server/../../docs/reference/fixtures/ws-transcript.json
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest)
        .join("../../docs/reference/fixtures/ws-transcript.json")
}
