//! Live mediasoup integration (live-host slice) — Linux + default features.
//! Real worker: supervisor spawn, router, transport create, E6 kill→respawn.
#![cfg(all(feature = "sfu-mediasoup", target_os = "linux"))]

use talkservo_core::ids::{PeerId, RoomId};
use talkservo_sfu::Sfu;

#[tokio::test]
async fn live_backend_creates_real_transport() {
    let sup = std::sync::Arc::new(talkservo_sfu::Supervisor::new());
    let sfu = talkservo_sfu::MediasoupSfu::new(sup);
    assert_eq!(sfu.backend(), "mediasoup");

    let room = RoomId::from("live-room");
    let peer = PeerId::from("live-peer");
    // real mediasoup worker spawns; a WebRtcTransport is created for real
    let info = sfu.create_transport(&room, &peer).await.expect("live transport");
    assert!(info.ice.is_object(), "ice parameters must be an object");
    assert!(info.dtls.is_object(), "dtls parameters must be an object");

    // second create for the same peer replaces (no leak) — behavior pin
    let _ = sfu.create_transport(&room, &peer).await.expect("second");

    // peer_left cascade drops the media objects (drop = close)
    sfu.peer_left(&room, &peer).await;
}

#[tokio::test]
async fn live_worker_e6_kill_notifies_and_respawns() {
    let sup = std::sync::Arc::new(talkservo_sfu::Supervisor::new());
    let mut restarts = sup.restart_rx();
    let sfu = talkservo_sfu::MediasoupSfu::new(sup.clone());
    let room = RoomId::from("e6-room");
    let _ = sfu.create_transport(&room, &PeerId::from("e6-peer")).await.expect("transport");

    sup.kill_worker().await;
    let note = restarts.recv().await.expect("restart notification");
    assert!(note.reason.contains("exited"));

    // supervisor respawns on next use
    let _ = sfu.create_transport(&room, &PeerId::from("after")).await.expect("respawned");
}
