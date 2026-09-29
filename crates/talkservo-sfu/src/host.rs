//! SFU host abstraction (modules/03) — the signaling server drives ALL media
//! through this trait. Two backends, exactly one active (compile-time gate in
//! lib.rs): `mediasoup` (Linux, live) and `stub` (tests / macOS CI).

use crate::error::SfuError;
use talkservo_core::floor::FloorState;
use talkservo_core::ids::{PeerId, RoomId};
use talkservo_core::wire::TransportInfo;

/// Media-activity state of a peer's uplink (E11 seam, plan-2 #3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityState {
    /// RTP flowing.
    Active,
    /// No RTP within the watchdog window.
    Silent,
}



/// Opaque producer id (wire carries it as JSON value; mediasoup `ProducerId`
/// stringifies). Kept as owned string to stay backend-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProducerId(pub String);

impl std::fmt::Display for ProducerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The SFU host surface (modules/03 §trait + plan-2 additions).
///
/// `apply_floor` consumes **state, not events** — replayable, self-healing
/// after R/W sequences; internal pause/resume diff (review M-4).
#[allow(async_fn_in_trait)]
pub trait Sfu {
    /// Create a WebRTC transport for `peer` in `room` (J-step 3).
    async fn create_transport(
        &self,
        room: &RoomId,
        peer: &PeerId,
    ) -> Result<TransportInfo, SfuError>;

    /// Complete DTLS handshake (J-step 5).
    async fn connect_transport(
        &self,
        room: &RoomId,
        peer: &PeerId,
        dtls: serde_json::Value,
    ) -> Result<(), SfuError>;

    /// Register an uplink producer, **created paused** — transmission gating
    /// (D13) is applied by `apply_floor`, never at produce time (J-step 6).
    async fn produce(
        &self,
        room: &RoomId,
        peer: &PeerId,
        rtp_parameters: serde_json::Value,
    ) -> Result<ProducerId, SfuError>;

    /// Open a downlink for `peer` consuming `producer_id` (J-step 7 client side).
    async fn consume(
        &self,
        room: &RoomId,
        peer: &PeerId,
        producer_id: &ProducerId,
    ) -> Result<serde_json::Value, SfuError>;

    /// Reconcile media with arbitration state — idempotent diff:
    /// resume granted holders' producers + matching consumers, pause the rest
    /// (D12/D13). Must be safe to call repeatedly with the same state.
    async fn apply_floor(&self, room: &RoomId, state: &FloorState);

    /// Peer gone — cascade close transports/producers/consumers (CM-4/#11).
    /// Queue purge is the SERVER's job (core domain); this is media-only.
    async fn peer_left(&self, room: &RoomId, peer: &PeerId);

    /// E11 watchdog input: is `peer`'s uplink producing RTP right now?
    async fn media_activity(&self, room: &RoomId, peer: &PeerId) -> ActivityState;

    /// E6 test hook: kill the underlying worker to exercise the W-sequence.
    /// Stub: no-op. Mediasoup: hard-kills the child (supervisor rebuilds).
    async fn kill_worker(&self);

    /// Backend label (observability).
    fn backend(&self) -> &'static str;
}
