//! mediasoup-backed SFU host (Linux, `sfu-mediasoup` feature).
//!
//! Skeleton slice this commit: `MediasoupSfu` wiring plan + Supervisor shell.
//! The full J-step 3-7 media plumbing lands with plan-2 Task 5's live tests —
//! this file exists so the default build carries a named backend; the trait
//! impl below is completed alongside those tests (single PR-sized unit).

use crate::error::SfuError;
use crate::host::{ActivityState, ProducerId, Sfu};
use talkservo_core::wire::TransportInfo;

use self::host_worker_events::WorkerRestarted;
use talkservo_core::floor::FloorState;
use talkservo_core::ids::{PeerId, RoomId};

/// Supervised mediasoup worker handle + per-room router registry.
///
/// W-sequence (modules/02 §W): on `worker.exited` → spawn replacement →
/// rebuild one Router per active room → signal server via channel. The
/// supervisor owns the `Worker`; `MediasoupSfu` clones cheap handles.
pub struct Supervisor {
    // plan-2 T5: worker_manager::WorkerManager + Worker + exit watcher task
    _private: (),
}

impl Supervisor {
    /// W-sequence notifications (server listens: rebuild + MediaRestart).
    pub fn restart_rx(
        &self,
    ) -> tokio::sync::mpsc::Receiver<WorkerRestarted> {
        unimplemented!("plan-2 Task 5 (live integration)")
    }
}

/// Placeholder module path for the restart event (kept out of `host` to avoid
/// a core→sfu dependency inversion).
pub mod host_worker_events {
    #[derive(Debug, Clone)]
    pub struct WorkerRestarted {
        pub reason: String,
    }
}

/// Live mediasoup host. Full trait impl lands with plan-2 Task 5 (live tests
/// on Linux); every method currently reports `SfuError::Other` with a task
/// pointer instead of pretending to work (verification honesty rule).
pub struct MediasoupSfu {
    _private: (),
}

impl MediasoupSfu {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for MediasoupSfu {
    fn default() -> Self {
        Self::new()
    }
}

impl Sfu for MediasoupSfu {
    async fn create_transport(
        &self,
        _room: &RoomId,
        _peer: &PeerId,
    ) -> Result<TransportInfo, SfuError> {
        Err(SfuError::Other(
            "mediasoup host lands with plan-2 Task 5 (live integration)".into(),
        ))
    }

    async fn connect_transport(
        &self,
        _room: &RoomId,
        _peer: &PeerId,
        _dtls: serde_json::Value,
    ) -> Result<(), SfuError> {
        Err(SfuError::Other(
            "mediasoup host lands with plan-2 Task 5".into(),
        ))
    }

    async fn produce(
        &self,
        _room: &RoomId,
        _peer: &PeerId,
        _rtp_parameters: serde_json::Value,
    ) -> Result<ProducerId, SfuError> {
        Err(SfuError::Other(
            "mediasoup host lands with plan-2 Task 5".into(),
        ))
    }

    async fn consume(
        &self,
        _room: &RoomId,
        _peer: &PeerId,
        _producer_id: &ProducerId,
    ) -> Result<serde_json::Value, SfuError> {
        Err(SfuError::Other(
            "mediasoup host lands with plan-2 Task 5".into(),
        ))
    }

    async fn apply_floor(&self, _room: &RoomId, _state: &FloorState) {
        // silent no-op until Task 5 wires the real diff
    }

    async fn peer_left(&self, _room: &RoomId, _peer: &PeerId) {
        // silent no-op until Task 5
    }

    async fn media_activity(&self, _room: &RoomId, _peer: &PeerId) -> ActivityState {
        // conservative default keeps the E11 watchdog inert until Task 5
        ActivityState::Active
    }

    async fn kill_worker(&self) {
        // no-op until Task 5
    }

    fn backend(&self) -> &'static str {
        "mediasoup"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_labels() {
        use crate::Sfu as _;
        assert_eq!(MediasoupSfu::new().backend(), "mediasoup");
    }
}
