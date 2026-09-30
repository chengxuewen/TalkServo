//! Latency taps (modules/05 §Obs, acceptance #7/#10):
//! grant_latency_ms on the grant path, media_recovery_ms on W-sequence.
#![cfg(feature = "stub-media")]

use futures_util::{SinkExt, StreamExt};
use talkservo_core::wire::SignalingMessage;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use talkservo_server::test_bridge;

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn ws_connect(url: &str) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(url).await.expect("connect");
    ws
}
async fn send_json(sink: &mut Ws, msg: &SignalingMessage) {
    sink.send(WsMessage::Text(serde_json::to_string(msg).expect("ser").into()))
        .await.expect("send");
}
async fn recv_msg(stream: &mut Ws) -> Option<SignalingMessage> {
    loop {
        let m = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next()).await.ok()??.ok()?;
        if let WsMessage::Text(t) = m {
            return serde_json::from_str(&t).ok();
        }
    }
}

#[tokio::test]
async fn grant_path_emits_latency_line() {
    // R-F21: assert the ring, not stdout (deterministic under the harness)
    let secret = format!("lat-secret-{}", std::process::id());
    let config = test_bridge::config_for_test(&secret);
    let app_state = test_bridge::test_app(config);
    let app = test_bridge::build_router(app_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

    let mut a = ws_connect(&format!("ws://{addr}/ws")).await;
    let jwt = test_bridge::issue_token(&secret, "lat-peer", "lat-room", talkservo_core::wire::Role::Field, 600);
    send_json(&mut a, &SignalingMessage::Join { v: 1, jwt }).await;
    for _ in 0..4 { recv_msg(&mut a).await.expect("burst"); }

    send_json(&mut a, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let mut granted = false;
    for _ in 0..6 {
        match recv_msg(&mut a).await {
            Some(SignalingMessage::FloorGranted { .. }) => { granted = true; break; }
            Some(_) => continue,
            None => break,
        }
    }
    assert!(granted);
    // R-F21: the latency line MUST be recorded — ring assertion, not stdout.
    let recs = talkservo_server::obs::drain_latency("grant_latency_ms");
    assert!(
        recs.iter().any(|r| r.room == "lat-room" && r.peer == "lat-peer"),
        "grant_latency_ms line for lat-room/lat-peer must exist: {recs:?}"
    );
}

#[tokio::test]
async fn grant_latency_covers_queued_promotion() {
    // queued peer's grant measures from ORIGINAL request (entry kept across
    // queueing): exclusive cap 1 — a queues, b queues behind, a releases →
    // b promoted → ONE grant_latency_ms line for b with the queued wait.
    let secret = format!("latq-secret-{}", std::process::id());
    let config = test_bridge::config_for_test(&secret);
    let app_state = test_bridge::test_app(config);
    let app = test_bridge::build_router(app_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

    let mut a = ws_connect(&format!("ws://{addr}/ws")).await;
    let mut b = ws_connect(&format!("ws://{addr}/ws")).await;
    let jwt_a = test_bridge::issue_token(&secret, "qa", "q-room", talkservo_core::wire::Role::Field, 600);
    let jwt_b = test_bridge::issue_token(&secret, "qb", "q-room", talkservo_core::wire::Role::Field, 600);
    send_json(&mut a, &SignalingMessage::Join { v: 1, jwt: jwt_a }).await;
    for _ in 0..4 { recv_msg(&mut a).await.expect("burst"); }
    send_json(&mut b, &SignalingMessage::Join { v: 1, jwt: jwt_b }).await;
    for _ in 0..4 { recv_msg(&mut b).await.expect("burst"); }

    // switch to exclusive so queuing happens
    send_json(&mut a, &SignalingMessage::ModeChange { mode: talkservo_core::wire::FloorMode::Exclusive }).await;
    let _ = recv_msg(&mut a).await; // idle echo (may or may not arrive)
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    send_json(&mut a, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let mut a_granted = false;
    for _ in 0..6 {
        match recv_msg(&mut a).await {
            Some(SignalingMessage::FloorGranted { .. }) => { a_granted = true; break; }
            Some(_) => continue,
            None => break,
        }
    }
    assert!(a_granted);
    send_json(&mut b, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let mut b_queued = false;
    for _ in 0..6 {
        match recv_msg(&mut b).await {
            Some(SignalingMessage::FloorQueued { .. }) => { b_queued = true; break; }
            Some(_) => continue,
            None => break,
        }
    }
    assert!(b_queued, "b must queue behind a (exclusive cap 1)");

    // a releases → b promoted; grant_latency_ms fires for b
    send_json(&mut a, &SignalingMessage::FloorRelease).await;
    let mut b_promoted = false;
    for _ in 0..8 {
        match recv_msg(&mut b).await {
            Some(SignalingMessage::FloorGranted { .. }) => { b_promoted = true; break; }
            Some(_) => continue,
            None => break,
        }
    }
    assert!(b_promoted, "b must be promoted after a releases");
    // R-F21: queued→promoted grant measures the FULL wait from the original
    // request — one record for qb must exist.
    let recs = talkservo_server::obs::drain_latency("grant_latency_ms");
    assert!(
        recs.iter().any(|r| r.room == "q-room" && r.peer == "qb"),
        "queued promotion must emit grant_latency_ms for qb: {recs:?}"
    );
}

