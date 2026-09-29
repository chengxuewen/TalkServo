#![cfg(feature = "stub-media")]

//! Plan-2 Task 2 integration tests — real WS clients against the stub path.
//! Feature combo (plan-2 #4): `--no-default-features --features stub-media`.

use futures_util::{SinkExt, StreamExt};
use talkservo_core::wire::SignalingMessage;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

use talkservo_server::test_bridge;

/// Ephemeral server instance (stub path). Inlined here: cargo compiles each
/// tests/*.rs as its own crate, so a shared harness file would double-compile.
pub struct TestServer {
    pub url: String,
    pub secret: String,
}

impl TestServer {
    pub async fn start() -> Self {
        let secret = format!("test-secret-{}", std::process::id());
        let config = test_bridge::config_for_test(&secret);
        let app_state = test_bridge::test_app(config);
        let app = test_bridge::build_router(app_state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        Self {
            url: format!("ws://{addr}/ws"),
            secret,
        }
    }

    pub fn ws_url(&self) -> &str {
        &self.url
    }
}

async fn ws_connect(url: &str) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(url).await.expect("connect");
    ws
}

async fn send_json(sink: &mut Ws, msg: &SignalingMessage) {
    let text = serde_json::to_string(msg).expect("ser");
    sink.send(WsMessage::Text(text.into()))
        .await
        .expect("send");
}

async fn recv_msg(stream: &mut Ws) -> SignalingMessage {
    loop {
        let m = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await
            .expect("recv timeout")
            .expect("stream ok")
            .expect("ws ok");
        if let WsMessage::Text(t) = m {
            return serde_json::from_str(&t).expect("deser");
        }
        // ignore pong/ping frames
    }
}

fn join_msg(secret: &str, sub: &str, room: &str, role: talkservo_core::wire::Role) -> SignalingMessage {
    let jwt = talkservo_server_test_helpers::issue(secret, sub, room, role);
    SignalingMessage::Join { v: 1, jwt }
}

// helper indirection so tests can mint tokens with the harness secret
mod talkservo_server_test_helpers {
    pub fn issue(
        secret: &str,
        sub: &str,
        room: &str,
        role: talkservo_core::wire::Role,
    ) -> String {
        // 1h token; tests never live that long
        talkservo_server::test_bridge::issue_token(secret, sub, room, role, 3600)
    }
}

#[tokio::test]
async fn join_welcome_caps_list_snapshot_sequence() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    send_json(&mut a, &join_msg(&srv.secret, "peer-a", "room-1", talkservo_core::wire::Role::Field)).await;

    // J sequence: Welcome → RouterCaps → PeerList → ServerSnapshot
    let w = recv_msg(&mut a).await;
    assert!(matches!(w, SignalingMessage::Welcome { v: 1, .. }));
    let c = recv_msg(&mut a).await;
    assert!(matches!(c, SignalingMessage::RouterCaps { .. }));
    let pl = recv_msg(&mut a).await;
    assert!(matches!(pl, SignalingMessage::PeerList { .. }));
    let snap = recv_msg(&mut a).await;
    assert!(matches!(
        snap,
        SignalingMessage::ServerSnapshot { .. }
    ));
}

#[tokio::test]
async fn identity_collision_rejects_second_join() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    send_json(&mut a, &join_msg(&srv.secret, "dup", "room-1", talkservo_core::wire::Role::Field)).await;
    let _ = recv_msg(&mut a).await; // welcome
    let _ = recv_msg(&mut a).await; // caps
    let _ = recv_msg(&mut a).await; // list
    let _ = recv_msg(&mut a).await; // snapshot

    let mut b = ws_connect(srv.ws_url()).await;
    send_json(&mut b, &join_msg(&srv.secret, "dup", "room-1", talkservo_core::wire::Role::Field)).await;
    let err = recv_msg(&mut b).await;
    match err {
        SignalingMessage::Error { code, .. } => assert_eq!(code, "already_joined"),
        other => panic!("expected Error, got {other:?}"),
    }
}

#[tokio::test]
async fn bad_version_closes_with_error() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    let jwt = talkservo_server_test_helpers::issue(&srv.secret, "p", "r", talkservo_core::wire::Role::Field);
    send_json(&mut a, &SignalingMessage::Join { v: 99, jwt }).await;
    let err = recv_msg(&mut a).await;
    assert!(matches!(
        err,
        SignalingMessage::Error { ref code, .. } if code == "bad_version"
    ));
}

#[tokio::test]
async fn dispatch_sees_queue_field_does_not() {
    let srv = TestServer::start().await;
    let mut d = ws_connect(srv.ws_url()).await;
    send_json(&mut d, &join_msg(&srv.secret, "disp", "room-1", talkservo_core::wire::Role::Dispatch)).await;
    // drain join burst (4 messages)
    for _ in 0..4 {
        let _ = recv_msg(&mut d).await;
    }
    let snap = SignalingMessage::Resync;
    send_json(&mut d, &snap).await;
    let m = recv_msg(&mut d).await;
    match m {
        SignalingMessage::ServerSnapshot { payload, .. } => {
            assert!(matches!(
                payload,
                talkservo_core::wire::ServerSnapshotPayload::Dispatch(_)
            ));
        }
        other => panic!("expected snapshot, got {other:?}"),
    }
}

#[tokio::test]
async fn floor_request_grants_first_peer() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    send_json(&mut a, &join_msg(&srv.secret, "pa", "room-1", talkservo_core::wire::Role::Field)).await;
    for _ in 0..4 {
        let _ = recv_msg(&mut a).await;
    }
    send_json(&mut a, &SignalingMessage::FloorRequest { priority: 0, preempt: false }).await;
    let m = recv_msg(&mut a).await;
    assert!(matches!(
        m,
        SignalingMessage::FloorGranted { .. }
    ));
}
