//! Plan-2 Task 4: E-matrix timer behaviors over the stub backend.
//! Intervals shortened via Config so tests run in seconds, not minutes.
#![cfg(feature = "stub-media")]

use futures_util::{SinkExt, StreamExt};
use talkservo_core::wire::SignalingMessage;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use std::sync::Arc;
use talkservo_server::test_bridge;

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct TestServer {
    url: String,
    secret: String,
    media: Arc<talkservo_sfu::StubSfu>,
}

impl TestServer {
    /// watchdog 200ms, grace 300ms, max-hold 400ms, refresh lead 1s, ttl 2s
    async fn start_fast() -> Self {
        let secret = format!("timers-secret-{}", std::process::id());
        let mut config = test_bridge::config_for_test(&secret);
        config.no_rtp_watchdog_ms = 200;
        config.floor_media_grace_ms = 300;
        config.floor_max_hold_ms = Some(400);
        config.jwt_ttl_s = 2;
        let media: Arc<talkservo_sfu::StubSfu> = Arc::new(talkservo_sfu::StubSfu::new());
        let app_state = Arc::new(test_bridge::AppState {
            config,
            rooms: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            media: media.clone(),
        });
        let app = test_bridge::build_router(app_state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        Self { url: format!("ws://{addr}/ws"), secret, media }
    }
    fn ws_url(&self) -> &str { &self.url }
}

async fn ws_connect(url: &str) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(url).await.expect("connect");
    ws
}

async fn send_json(sink: &mut Ws, msg: &SignalingMessage) {
    sink.send(WsMessage::Text(serde_json::to_string(msg).expect("ser").into()))
        .await.expect("send");
}

/// Timer ticks interleave protocol messages (TokenRefresh push runs on a 1s
/// sweep) — tests receive-until a predicate matches, within a message budget.
async fn recv_until(
    stream: &mut Ws,
    pred: impl Fn(&SignalingMessage) -> bool,
    budget: usize,
) -> Option<SignalingMessage> {
    for _ in 0..budget {
        match recv_msg(stream).await {
            Some(msg) if pred(&msg) => return Some(msg),
            Some(_) => continue,
            None => return None, // silence — stop burning the budget
        }
    }
    None
}

/// One text frame or None on 5s silence (callers decide whether that's fatal).
async fn recv_msg(stream: &mut Ws) -> Option<SignalingMessage> {
    loop {
        let m = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await // Result<Option<Result<frame>>>
            .ok()?  // silence → None
            ?       // stream ended
            .ok()?; // ws error → None
        if let WsMessage::Text(t) = m {
            return serde_json::from_str(&t).ok();
        }
    }
}

async fn join_and_drain(ws: &mut Ws, srv: &TestServer, sub: &str) {
    let jwt = test_bridge::issue_token(&srv.secret, sub, "room-1", talkservo_core::wire::Role::Field, 3600);
    send_json(ws, &SignalingMessage::Join { v: 1, jwt }).await;
    for _ in 0..4 { recv_msg(ws).await.expect("burst"); }
}

#[tokio::test]
async fn e11_silence_past_grace_releases_and_promotes() {
    // no max-hold: the E11 release must be the only promotion source here
    let mut srv = TestServer::start_fast().await;
    srv.media = srv.media.clone();
    {
        // rebuild with max-hold off by restarting (simplest: new instance)
    }
    let srv = {
        let secret = format!("e11-secret-{}", std::process::id());
        let mut config = test_bridge::config_for_test(&secret);
        config.no_rtp_watchdog_ms = 200;
        config.floor_media_grace_ms = 300;
        config.floor_max_hold_ms = None; // OFF
        let media: Arc<talkservo_sfu::StubSfu> = Arc::new(talkservo_sfu::StubSfu::new());
        let app_state = Arc::new(test_bridge::AppState {
            config,
            rooms: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            media: media.clone(),
        });
        let app = test_bridge::build_router(app_state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        TestServer { url: format!("ws://{addr}/ws"), secret, media }
    };
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "holder").await;
    let mut b = ws_connect(srv.ws_url()).await;
    {
        let jwt = test_bridge::issue_token(&srv.secret, "queued", "room-1", talkservo_core::wire::Role::Field, 3600);
        send_json(&mut b, &SignalingMessage::Join { v: 1, jwt }).await;
        for i in 0..4 {
            let m = recv_msg(&mut b).await.expect("burst msg");
            eprintln!("b burst[{i}]: {m:?}");
            let _ = m;
        }
    }

    // switch to Exclusive (default room mode is Hybrid, cap 2 — both would
    // grant straight away); a drains the mode_change echo, b sees nothing
    send_json(&mut a, &SignalingMessage::ModeChange { mode: talkservo_core::wire::FloorMode::Exclusive }).await;
    let _ = recv_until(&mut a, |m| matches!(m, SignalingMessage::FloorIdle { .. }), 6).await;

    // both request; exclusive cap 1: holder granted, queued queued
    send_json(&mut a, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let g = recv_until(&mut a, |m| matches!(m, SignalingMessage::FloorGranted { .. }), 4).await;
    assert!(g.is_some());
    send_json(&mut b, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let q = recv_until(&mut b, |m| matches!(m, SignalingMessage::FloorQueued { .. }), 10).await;
    assert!(q.is_some());

    // script holder's uplink silent; watchdog(200ms)+grace(300ms) → MediaDown
    srv.media.set_activity(
        &talkservo_core::ids::RoomId::from("room-1"),
        &talkservo_core::ids::PeerId::from("holder"),
        talkservo_sfu::ActivityState::Silent,
    );

    // holder sees MediaFailed + FloorIdle; queued sees promote FloorGranted
    // (and its own queued position rebroadcasts may arrive — filter)
    let failed = recv_until(&mut a, |m| matches!(m, SignalingMessage::MediaFailed { .. }), 8).await;
    assert!(failed.is_some(), "holder must see MediaFailed");
    let promoted = recv_until(&mut b, |m| matches!(m, SignalingMessage::FloorGranted { .. }), 8).await;
    assert!(promoted.is_some(), "queued peer must be promoted after E11 release");
}

#[tokio::test]
async fn max_hold_timeout_releases_floor() {
    let srv = TestServer::start_fast().await;
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "longholder").await;

    send_json(&mut a, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let granted = recv_until(&mut a, |m| matches!(m, SignalingMessage::FloorGranted { .. }), 4).await;
    assert!(granted.is_some(), "grant must arrive (noise-tolerant recv)");

    // max-hold 400ms → Timeout → FloorIdle(reason=timeout)
    let idle = recv_until(
        &mut a,
        |m| matches!(m, SignalingMessage::FloorIdle { reason: Some(r), .. } if r == "timeout"),
        6,
    )
    .await;
    assert!(idle.is_some(), "max-hold must emit FloorIdle(timeout)");
}

#[tokio::test]
async fn token_refresh_pushed_proactively() {
    let srv = TestServer::start_fast().await;
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "refresher").await;

    // ttl 2s, lead 1s → refresh arrives within ~1-2s
    let got = recv_until(&mut a, |m| matches!(m, SignalingMessage::TokenRefresh { .. }), 8).await;
    assert!(got.is_some(), "TokenRefresh must be pushed before expiry");
}

#[tokio::test]
async fn cooldown_second_rapid_request_denied_rate_limited() {
    let srv = TestServer::start_fast().await;
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "spammer").await;

    // NOTE: server cooldown is 500ms fixed (Config::for_test); two rapid sends
    send_json(&mut a, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let first = recv_until(&mut a, |m| matches!(m, SignalingMessage::FloorGranted { .. }), 4).await;
    assert!(first.is_some());

    // second request within cooldown → RateLimited (deterministic: cooldown
    // gate runs before the already-granted idempotency check)
    send_json(&mut a, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let denied = recv_until(
        &mut a,
        |m| matches!(m, SignalingMessage::FloorDenied { reason: talkservo_core::wire::DenyReason::RateLimited, .. }),
        4,
    )
    .await;
    assert!(denied.is_some(), "rapid re-request must hit the cooldown gate");
}
