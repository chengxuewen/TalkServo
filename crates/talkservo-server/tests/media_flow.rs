//! Plan-2 Task 3 integration: media orchestration over the stub backend.
#![cfg(feature = "stub-media")]

use futures_util::{SinkExt, StreamExt};
use talkservo_core::wire::SignalingMessage;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use std::sync::Arc;
use talkservo_server::test_bridge;

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct TestServer {
    url: String,
    secret: String,
    media: Arc<talkservo_sfu::StubSfu>,
}

impl TestServer {
    async fn start() -> Self {
        let secret = format!("media-secret-{}", std::process::id());
        let config = test_bridge::config_for_test(&secret);
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
        .await
        .expect("send");
}

async fn recv_msg(stream: &mut Ws) -> SignalingMessage {
    loop {
        let m = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await.expect("timeout")
            .expect("stream").expect("ws");
        if let WsMessage::Text(t) = m {
            return serde_json::from_str(&t).expect("deser");
        }
    }
}

async fn join_and_drain(ws: &mut Ws, srv: &TestServer, sub: &str, role: talkservo_core::wire::Role) {
    let jwt = test_bridge::issue_token(&srv.secret, sub, "room-1", role, 3600);
    send_json(ws, &SignalingMessage::Join { v: 1, jwt }).await;
    // J burst = Welcome, RouterCaps, PeerList, ServerSnapshot (PeerJoined goes
    // to OTHERS only — broadcast_except self)
    for _ in 0..4 {
        let _ = recv_msg(ws).await;
    }
}

#[tokio::test]
async fn transport_create_connect_produce_flow() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "peer-a", talkservo_core::wire::Role::Field).await;

    // J-step 3: TransportCreate → TransportInfo
    send_json(&mut a, &SignalingMessage::TransportCreate).await;
    assert!(matches!(recv_msg(&mut a).await, SignalingMessage::TransportInfo { .. }));

    // J-step 5: connect
    send_json(&mut a, &SignalingMessage::TransportConnect { dtls: serde_json::json!({}) }).await;
    // no reply on success

    // J-step 6: Produce → ProduceOk
    send_json(&mut a, &SignalingMessage::Produce { rtp_parameters: serde_json::json!({}) }).await;
    assert!(matches!(recv_msg(&mut a).await, SignalingMessage::ProduceOk { .. }));

    // sfu saw the calls
    let calls = srv.media.calls();
    assert!(calls.iter().any(|c| matches!(c, talkservo_sfu::stub::Call::Produce { peer, .. } if peer == "peer-a")));
}

#[tokio::test]
async fn produce_before_connect_rejected() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "peer-b", talkservo_core::wire::Role::Field).await;

    send_json(&mut a, &SignalingMessage::Produce { rtp_parameters: serde_json::json!({}) }).await;
    match recv_msg(&mut a).await {
        SignalingMessage::Error { code, .. } => assert_eq!(code, "bad_order"),
        other => panic!("expected bad_order, got {other:?}"),
    }
}

#[tokio::test]
async fn transport_guardrail_capacity_reject() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "peer-c", talkservo_core::wire::Role::Field).await;

    // fill the room's guardrail via direct transport-count seeding is not
    // exposed — drive it: 50 creates through the real path (default guardrail)
    for i in 0..50 {
        send_json(&mut a, &SignalingMessage::TransportCreate).await;
        let m = recv_msg(&mut a).await;
        assert!(matches!(m, SignalingMessage::TransportInfo { .. }), "create {i} failed");
    }
    // 51st → media_capacity
    send_json(&mut a, &SignalingMessage::TransportCreate).await;
    match recv_msg(&mut a).await {
        SignalingMessage::Error { code, .. } => assert_eq!(code, "media_capacity"),
        other => panic!("expected media_capacity, got {other:?}"),
    }
}

#[tokio::test]
async fn e4_transport_timeout_emits_media_failed() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    join_and_drain(&mut a, &srv, "peer-d", talkservo_core::wire::Role::Field).await;

    // script E4: next transport for peer-d fails
    srv.media.fail_next_transport(&talkservo_core::ids::PeerId::from("peer-d"));
    send_json(&mut a, &SignalingMessage::TransportCreate).await;
    // first response: MediaFailed broadcast (to self, sole member)
    let m = recv_msg(&mut a).await;
    assert!(matches!(m, SignalingMessage::MediaFailed { .. }), "got {m:?}");
    // then the error
    match recv_msg(&mut a).await {
        SignalingMessage::Error { code, .. } => assert_eq!(code, "transport_timeout"),
        other => panic!("expected transport_timeout, got {other:?}"),
    }
}
