//! Live mediasoup integration (plan-2 T5) — Linux + default features only.
//! Real worker: supervisor spawn, transports, produce silence, E6 kill → W-seq.
#![cfg(all(feature = "sfu-mediasoup", target_os = "linux"))]

use talkservo_core::ids::{PeerId, RoomId};
use talkservo_sfu::Sfu;

#[tokio::test]
async fn mediasoup_backend_labels_and_reports_honest_errors() {
    // MediasoupSfu skeleton: every media call reports the Task-5 pointer until
    // the full wiring lands; the labels must be honest (no fake success).
    let sfu = talkservo_sfu::MediasoupSfu::new();
    assert_eq!(sfu.backend(), "mediasoup");

    let room = RoomId::from("live-room");
    let peer = PeerId::from("live-peer");
    let r = sfu.create_transport(&room, &peer).await;
    match r {
        Err(talkservo_sfu::SfuError::Other(m)) => {
            assert!(m.contains("Task 5"), "error must carry the task pointer: {m}");
        }
        Ok(_) => panic!("skeleton must not fake success"),
        Err(e) => panic!("unexpected error kind: {e}"),
    }
}
