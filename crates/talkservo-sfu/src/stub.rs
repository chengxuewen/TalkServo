//! Stub SFU host — no-op backend with a recorded call log and scriptable
//! media activity. This is the test harness for the whole server suite
//! (plan-2 #4: stub suite runs on every machine; live suite = Linux).

use crate::error::SfuError;
use crate::host::{ActivityState, ProducerId, Sfu};
use talkservo_core::wire::TransportInfo;
use crate::stub_internal::{MediaPeer, StubState};
use std::sync::Mutex;
use talkservo_core::floor::FloorState;
use talkservo_core::ids::{PeerId, RoomId};

/// Recorded host calls — assertions read this instead of poking media objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    CreateTransport { room: String, peer: String },
    ConnectTransport { room: String, peer: String },
    Produce { room: String, peer: String },
    Consume { room: String, peer: String, producer: String },
    /// apply_floor diff outcome: which peers ended resumed / paused.
    ApplyFloor { room: String, resumed: Vec<String>, paused: Vec<String> },
    PeerLeft { room: String, peer: String },
}

/// Stub host: deterministic, `Mutex`-guarded world.
#[derive(Default)]
pub struct StubSfu {
    state: Mutex<StubState>,
}

impl StubSfu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of recorded calls (for assertions).
    pub fn calls(&self) -> Vec<Call> {
        self.state.lock().expect("stub state").calls.clone()
    }

    /// Clear the call log (between test phases).
    pub fn clear_calls(&self) {
        self.state.lock().expect("stub state").calls.clear();
    }

    /// Script the E11 answer for a peer (default: Active once produced).
    pub fn set_activity(&self, room: &RoomId, peer: &PeerId, activity: ActivityState) {
        let mut st = self.state.lock().expect("stub state");
        st.activity
            .entry(room.to_string())
            .or_default()
            .insert(peer.to_string(), activity);
    }

    /// E4 helper: make the next `create_transport` for `peer` time out.
    pub fn fail_next_transport(&self, peer: &PeerId) {
        self.state
            .lock()
            .expect("stub state")
            .fail_transport
            .push(peer.to_string());
    }

    /// E10 helper: set the live transport count (guardrail probing).
    pub fn set_transport_count(&self, room: &RoomId, n: usize) {
        let mut st = self.state.lock().expect("stub state");
        st.transport_count.insert(room.to_string(), n);
    }

    /// E6 helper (stub): record that a kill was requested (no real worker).
    pub fn killed_workers(&self) -> u32 {
        self.state.lock().expect("stub state").kills
    }

    fn with_state<R>(&self, f: impl FnOnce(&mut StubState) -> R) -> R {
        let mut st = self.state.lock().expect("stub state");
        f(&mut st)
    }
}

impl Sfu for StubSfu {
    async fn create_transport(
        &self,
        room: &RoomId,
        peer: &PeerId,
    ) -> Result<TransportInfo, SfuError> {
        self.with_state(|st| {
            if st.fail_transport.iter().any(|p| p == &*peer.0) {
                st.fail_transport.retain(|p| p != &*peer.0);
                return Err(SfuError::TransportTimeout {
                    peer: peer.to_string(),
                });
            }
            let count = st.transport_count.entry(room.to_string()).or_insert(0);
            *count += 1;
            st.rooms.entry(room.to_string()).or_default().insert(
                peer.to_string(),
                MediaPeer::default(),
            );
            st.calls.push(Call::CreateTransport {
                room: room.to_string(),
                peer: peer.to_string(),
            });
            Ok(TransportInfo {
                ice: serde_json::json!({"ufrag": format!("stub-{peer}")}),
                dtls: serde_json::json!({"fingerprint": "stub"}),
                addrs: serde_json::json!([]),
            })
        })
    }

    async fn connect_transport(
        &self,
        room: &RoomId,
        peer: &PeerId,
        _dtls: serde_json::Value,
    ) -> Result<(), SfuError> {
        self.with_state(|st| {
            let peers = st
                .rooms
                .get_mut(room.0.as_ref())
                .ok_or_else(|| SfuError::Other("no such room".into()))?;
            let mp = peers
                .get_mut(peer.0.as_ref())
                .ok_or_else(|| SfuError::Other("no transport for peer".into()))?;
            mp.connected = true;
            st.calls.push(Call::ConnectTransport {
                room: room.to_string(),
                peer: peer.to_string(),
            });
            Ok(())
        })
    }

    async fn produce(
        &self,
        room: &RoomId,
        peer: &PeerId,
        _rtp_parameters: serde_json::Value,
    ) -> Result<ProducerId, SfuError> {
        self.with_state(|st| {
            let peers = st
                .rooms
                .get_mut(room.0.as_ref())
                .ok_or_else(|| SfuError::Other("no such room".into()))?;
            let mp = peers
                .get_mut(peer.0.as_ref())
                .ok_or_else(|| SfuError::Other("no transport for peer".into()))?;
            let id = format!("prod-{}-{}", peer, mp.producers.len() + 1);
            mp.producers.insert(id.clone(), false); // created PAUSED (D13)
            st.calls.push(Call::Produce {
                room: room.to_string(),
                peer: peer.to_string(),
            });
            Ok(ProducerId(id))
        })
    }

    async fn consume(
        &self,
        room: &RoomId,
        peer: &PeerId,
        producer_id: &ProducerId,
    ) -> Result<serde_json::Value, SfuError> {
        self.with_state(|st| {
            st.calls.push(Call::Consume {
                room: room.to_string(),
                peer: peer.to_string(),
                producer: producer_id.0.clone(),
            });
            let mp = st
                .rooms
                .entry(room.to_string())
                .or_default()
                .entry(peer.to_string())
                .or_default();
            mp.consumers.insert(producer_id.0.clone(), true);
            Ok(serde_json::json!({
                "producerId": producer_id.0,
                "rtpParameters": {"codecs": [], "headerExtensions": []}
            }))
        })
    }

    async fn apply_floor(&self, room: &RoomId, state: &FloorState) {
        self.with_state(|st| {
            let mut resumed = Vec::new();
            let mut paused = Vec::new();
            if let Some(peers) = st.rooms.get_mut(room.0.as_ref()) {
                for (peer_str, mp) in peers.iter_mut() {
                    let granted = state
                        .grants()
                        .iter()
                        .any(|g| g.0.as_ref() == peer_str);
                    // set desired pause state wholesale (idempotent diff target:
                    // granted → resumed, everyone else paused — D13)
                    for flag in mp.producers.values_mut() {
                        *flag = !granted;
                    }
                    if granted {
                        resumed.push(peer_str.clone());
                    } else if !mp.producers.is_empty() {
                        paused.push(peer_str.clone());
                    }
                }
            }
            st.calls.push(Call::ApplyFloor {
                room: room.to_string(),
                resumed,
                paused,
            });
        })
    }

    async fn peer_left(&self, room: &RoomId, peer: &PeerId) {
        self.with_state(|st| {
            if let Some(peers) = st.rooms.get_mut(room.0.as_ref()) {
                peers.remove(peer.0.as_ref());
            }
            if let Some(act) = st.activity.get_mut(room.0.as_ref()) {
                act.remove(peer.0.as_ref());
            }
            st.calls.push(Call::PeerLeft {
                room: room.to_string(),
                peer: peer.to_string(),
            });
        })
    }

    async fn media_activity(&self, room: &RoomId, peer: &PeerId) -> ActivityState {
        self.with_state(|st| {
            st.activity
                .get(room.0.as_ref())
                .and_then(|m| m.get(peer.0.as_ref()))
                .copied()
                .unwrap_or(ActivityState::Active)
        })
    }

    async fn kill_worker(&self) {
        self.with_state(|st| st.kills += 1);
    }

    fn backend(&self) -> &'static str {
        "stub"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkservo_core::wire::FloorMode;

    fn rid(s: &str) -> RoomId {
        RoomId::from(s)
    }
    fn pid(s: &str) -> PeerId {
        PeerId::from(s)
    }

    #[test]
    fn stub_create_produce_apply_floor_diff() {
        let sfu = StubSfu::new();
        let room = rid("r1");
        let (a, b) = (pid("a"), pid("b"));

        sfu.create_transport(&room, &a).await_or_block().expect("ta");
        sfu.create_transport(&room, &b).await_or_block().expect("tb");
        sfu.connect_transport(&room, &a, serde_json::json!({}))
            .await_or_block()
            .expect("connect");
        let prod = sfu
            .produce(&room, &a, serde_json::json!({}))
            .await_or_block()
            .expect("produce");
        assert_eq!(prod.0, "prod-a-1");

        // a granted → resumed; b paused (has no producers — not in either list)
        let state = FloorState::initial_with_generation(FloorMode::Exclusive, 5);
        let (granted_state, _) = state.apply(
            &talkservo_core::floor::FloorEvent::Request {
                peer: a.clone(),
                priority: 0,
                preempt: false,
            },
            &talkservo_core::floor::FloorLimits::default(),
        );
        smol_block(sfu.apply_floor(&room, &granted_state));

        let calls = sfu.calls();
        assert!(calls.contains(&Call::ApplyFloor {
            room: "r1".into(),
            resumed: vec!["a".into()],
            paused: vec![],
        }));

        // E11: default activity Active; script Silent
        sfu.set_activity(&room, &a, ActivityState::Silent);
        smol_block(async {
            assert_eq!(
                sfu.media_activity(&room, &a).await,
                ActivityState::Silent
            );
        });

        // cascade: peer_left removes the media peer
        smol_block(sfu.peer_left(&room, &b));
        assert!(calls_contains(&sfu, |c| matches!(c, Call::PeerLeft { peer, .. } if peer == "b")));

        // E6 hook counts kills
        smol_block(sfu.kill_worker());
        assert_eq!(sfu.killed_workers(), 1);

        // E10: transport count bookkeeping
        sfu.set_transport_count(&room, 50);
        smol_block(async {
            let r = sfu.create_transport(&room, &pid("zz")).await;
            assert!(r.is_ok(), "stub does not enforce guardrail — server does");
        });

        // E4: scripted transport failure
        sfu.fail_next_transport(&pid("e4"));
        smol_block(async {
            let r = sfu.create_transport(&room, &pid("e4")).await;
            assert!(matches!(r, Err(SfuError::TransportTimeout { .. })));
        });
    }

    // -- tiny block-on helpers (no tokio dev-dep in this crate) -------------
    trait AwaitBlock {
        type Out;
        fn await_or_block(self) -> Self::Out;
    }

    impl<F: std::future::Future> AwaitBlock for F {
        type Out = F::Output;
        fn await_or_block(self) -> F::Output {
            pollster_block(self)
        }
    }

    fn pollster_block<F: std::future::Future>(fut: F) -> F::Output {
        // minimal executor: busy-poll — fine for stub tests (no I/O waits)
        let mut fut = Box::pin(fut);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            match fut.as_mut().poll(&mut cx) {
                std::task::Poll::Ready(v) => return v,
                std::task::Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn smol_block<F: std::future::Future>(fut: F) -> F::Output {
        pollster_block(fut)
    }

    fn calls_contains(sfu: &StubSfu, pred: impl Fn(&Call) -> bool) -> bool {
        sfu.calls().iter().any(|c| pred(c))
    }
}
