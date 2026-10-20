#![cfg(feature = "stub-media")]

//! Plan-2 Task 2 integration tests — real WS clients against the stub path.
//! Feature combo (plan-2 #4): `--no-default-features --features stub-media`.

use futures_util::{SinkExt, StreamExt};
use talkservo_core::wire::SignalingMessage;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
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
    let (ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("connect");
    ws
}

async fn send_json(sink: &mut Ws, msg: &SignalingMessage) {
    let text = serde_json::to_string(msg).expect("ser");
    sink.send(WsMessage::Text(text.into())).await.expect("send");
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

fn join_msg(
    secret: &str,
    sub: &str,
    room: &str,
    role: talkservo_core::wire::Role,
) -> SignalingMessage {
    let jwt = talkservo_server_test_helpers::issue(secret, sub, room, role);
    SignalingMessage::Join { v: 1, jwt }
}

// helper indirection so tests can mint tokens with the harness secret
mod talkservo_server_test_helpers {
    pub fn issue(secret: &str, sub: &str, room: &str, role: talkservo_core::wire::Role) -> String {
        // 1h token; tests never live that long
        talkservo_server::test_bridge::issue_token(secret, sub, room, role, 3600)
    }
}

#[tokio::test]
async fn join_welcome_caps_list_snapshot_sequence() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    send_json(
        &mut a,
        &join_msg(
            &srv.secret,
            "peer-a",
            "room-1",
            talkservo_core::wire::Role::Field,
        ),
    )
    .await;

    // J sequence: Welcome → RouterCaps → PeerList → ServerSnapshot
    let w = recv_msg(&mut a).await;
    assert!(matches!(w, SignalingMessage::Welcome { v: 1, .. }));
    let c = recv_msg(&mut a).await;
    assert!(matches!(c, SignalingMessage::RouterCaps { .. }));
    let pl = recv_msg(&mut a).await;
    assert!(matches!(pl, SignalingMessage::PeerList { .. }));
    let snap = recv_msg(&mut a).await;
    assert!(matches!(snap, SignalingMessage::ServerSnapshot { .. }));
}

#[tokio::test]
async fn identity_collision_rejects_second_join() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    send_json(
        &mut a,
        &join_msg(
            &srv.secret,
            "dup",
            "room-1",
            talkservo_core::wire::Role::Field,
        ),
    )
    .await;
    let _ = recv_msg(&mut a).await; // welcome
    let _ = recv_msg(&mut a).await; // caps
    let _ = recv_msg(&mut a).await; // list
    let _ = recv_msg(&mut a).await; // snapshot

    let mut b = ws_connect(srv.ws_url()).await;
    send_json(
        &mut b,
        &join_msg(
            &srv.secret,
            "dup",
            "room-1",
            talkservo_core::wire::Role::Field,
        ),
    )
    .await;
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
    let jwt = talkservo_server_test_helpers::issue(
        &srv.secret,
        "p",
        "r",
        talkservo_core::wire::Role::Field,
    );
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
    send_json(
        &mut d,
        &join_msg(
            &srv.secret,
            "disp",
            "room-1",
            talkservo_core::wire::Role::Dispatch,
        ),
    )
    .await;
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
async fn mute_takes_effect_live_and_survives_resync_snapshot() {
    // S0.1: the snapshot muted field was hardcoded empty — R1/reconnect lost
    // the mute roster on the floor. Peer ids come from the token sub.
    let srv = TestServer::start().await;
    let mut d = ws_connect(srv.ws_url()).await;
    send_json(
        &mut d,
        &join_msg(
            &srv.secret,
            "disp",
            "room-m",
            talkservo_core::wire::Role::Dispatch,
        ),
    )
    .await;
    for _ in 0..4 {
        let _ = recv_msg(&mut d).await;
    }
    let mut v = ws_connect(srv.ws_url()).await;
    send_json(
        &mut v,
        &join_msg(
            &srv.secret,
            "quiet-guy",
            "room-m",
            talkservo_core::wire::Role::Field,
        ),
    )
    .await;
    for _ in 0..4 {
        let _ = recv_msg(&mut v).await;
    }
    // S7.5 A2 rewrote Rule 8: holding the floor is no longer a PRECONDITION for
    // mute to stick. The case kept here is the one that matters most - mute must
    // take an occupied seat away, not just mark a name.
    send_json(
        &mut v,
        &SignalingMessage::FloorRequest {
            priority: 0,
            preempt: false,
        },
    )
    .await;
    let _ = recv_msg(&mut v).await; // FloorGranted
    send_json(
        &mut d,
        &SignalingMessage::MuteSet {
            peer: talkservo_core::ids::PeerId::from("quiet-guy"),
            on: true,
        },
    )
    .await;
    // S7.5 A1: mute is no longer silent. The victim sees the seat go away AND the
    // roster change live - the floor message plus the role-scoped snapshot push the
    // server now makes for MuteSet (room.rs). No resync, no new wire variant.
    let mut saw_idle = false;
    let mut saw_roster = false;
    for _ in 0..4 {
        if saw_idle && saw_roster {
            break;
        }
        match recv_msg(&mut v).await {
            SignalingMessage::FloorIdle { .. } => saw_idle = true,
            SignalingMessage::ServerSnapshot {
                payload: talkservo_core::wire::ServerSnapshotPayload::Field(fs),
                ..
            } => saw_roster = fs.muted.iter().any(|p| p.0.as_ref() == "quiet-guy"),
            _ => {}
        }
    }
    assert!(
        saw_idle && saw_roster,
        "mute must take the floor and show the roster live, unsolicited"
    );

    // S7.5 A1: a muted member who presses again is refused WITH A REASON, reusing
    // the existing Unauthorized variant (no new wire value - D16 compat holds).
    // server's request cooldown (500ms default) is a legitimate EARLIER gate, so the
    // test waits it out - otherwise this line would be measuring RateLimited.
    tokio::time::sleep(std::time::Duration::from_millis(650)).await;
    send_json(
        &mut v,
        &SignalingMessage::FloorRequest {
            priority: 0,
            preempt: false,
        },
    )
    .await;
    let denied = loop {
        let m = recv_msg(&mut v).await;
        if matches!(m, SignalingMessage::FloorDenied { .. }) {
            break m;
        }
    };
    assert!(
        matches!(
            denied,
            SignalingMessage::FloorDenied {
                reason: talkservo_core::wire::DenyReason::Unauthorized,
                ..
            }
        ),
        "muted request must be refused as unauthorized, got {denied:?}"
    );
    send_json(&mut d, &SignalingMessage::Resync).await;
    let m = loop {
        let m = recv_msg(&mut d).await;
        if matches!(m, SignalingMessage::ServerSnapshot { .. }) {
            break m;
        }
    };
    match m {
        SignalingMessage::ServerSnapshot {
            payload: talkservo_core::wire::ServerSnapshotPayload::Dispatch(ds),
            ..
        } => {
            assert!(
                ds.field.muted.iter().any(|p| p.0.as_ref() == "quiet-guy"),
                "dispatch snapshot must carry the mute roster: {:?}",
                ds.field.muted
            );
        }
        other => panic!("expected dispatch snapshot, got {other:?}"),
    }
    send_json(&mut v, &SignalingMessage::Resync).await;
    let mv = loop {
        let m = recv_msg(&mut v).await;
        if matches!(m, SignalingMessage::ServerSnapshot { .. }) {
            break m;
        }
    };
    match mv {
        SignalingMessage::ServerSnapshot {
            payload: talkservo_core::wire::ServerSnapshotPayload::Field(fs),
            ..
        } => {
            assert!(
                fs.muted.iter().any(|p| p.0.as_ref() == "quiet-guy"),
                "field snapshot muted roster lost (R1 mute-state bug): {:?}",
                fs.muted
            );
        }
        other => panic!("expected field snapshot, got {other:?}"),
    }

    // S7.5 A3: un-mute restores the PRIVILEGE, never the floor. Nothing may arrive
    // as a grant - the peer presses again, and that request carries its own
    // priority, so no priority state has to be remembered or replayed.
    send_json(
        &mut d,
        &SignalingMessage::MuteSet {
            peer: talkservo_core::ids::PeerId::from("quiet-guy"),
            on: false,
        },
    )
    .await;
    let auto_grant = tokio::time::timeout(std::time::Duration::from_millis(400), async {
        loop {
            if matches!(
                recv_msg(&mut v).await,
                SignalingMessage::FloorGranted { .. }
            ) {
                return true;
            }
        }
    })
    .await;
    assert!(
        auto_grant.is_err(),
        "un-mute must not hand the floor back (S7.5 A3)"
    );

    // one more cooldown wait (500ms default): the previous press is the denied one,
    // and a RateLimited answer here would make the assertion below measure the wrong
    // thing.
    tokio::time::sleep(std::time::Duration::from_millis(650)).await;

    send_json(
        &mut v,
        &SignalingMessage::FloorRequest {
            priority: 3,
            preempt: false,
        },
    )
    .await;
    let regranted = loop {
        let m = recv_msg(&mut v).await;
        match &m {
            SignalingMessage::FloorGranted { .. } => break m,
            SignalingMessage::FloorDenied { reason, .. } => {
                panic!("freed member's request refused again ({reason:?})")
            }
            _ => {} // the un-mute snapshot may still be in flight
        }
    };
    assert!(
        matches!(regranted, SignalingMessage::FloorGranted { .. }),
        "a freed member can hold again by pressing"
    );
}

#[tokio::test]
async fn floor_request_grants_first_peer() {
    let srv = TestServer::start().await;
    let mut a = ws_connect(srv.ws_url()).await;
    send_json(
        &mut a,
        &join_msg(
            &srv.secret,
            "pa",
            "room-1",
            talkservo_core::wire::Role::Field,
        ),
    )
    .await;
    for _ in 0..4 {
        let _ = recv_msg(&mut a).await;
    }
    send_json(
        &mut a,
        &SignalingMessage::FloorRequest {
            priority: 0,
            preempt: false,
        },
    )
    .await;
    let m = recv_msg(&mut a).await;
    assert!(matches!(m, SignalingMessage::FloorGranted { .. }));
}

#[tokio::test]
async fn mute_naming_an_absent_peer_is_refused_visibly() {
    // S7.5 A2: the target-must-exist check lives in the server (the core keeps no
    // roster), and an unknown id is ANSWERED, not swallowed (R4 fail-visible - a
    // silently dropped admin command is how the mute switch got its reputation).
    let srv = TestServer::start().await;
    let mut d = ws_connect(srv.ws_url()).await;
    send_json(
        &mut d,
        &join_msg(
            &srv.secret,
            "disp-x",
            "room-ghost",
            talkservo_core::wire::Role::Dispatch,
        ),
    )
    .await;
    for _ in 0..4 {
        let _ = recv_msg(&mut d).await;
    }
    send_json(
        &mut d,
        &SignalingMessage::MuteSet {
            peer: talkservo_core::ids::PeerId::from("nobody"),
            on: true,
        },
    )
    .await;
    let err = loop {
        let m = recv_msg(&mut d).await;
        if matches!(m, SignalingMessage::Error { .. }) {
            break m;
        }
    };
    assert!(
        matches!(&err, SignalingMessage::Error { code, .. } if code.as_str() == "unknown_peer"),
        "absent mute target must be refused visibly, got {err:?}"
    );
}
