//! The signaling wire contract (modules/02 table) — closed enums, additive-only
//! evolution, snake_case tags. This is the single source of truth; plan-3's TS
//! unions are generated from it (schema artifact step).

use crate::ids::{PeerId, RoomId};
use serde::{Deserialize, Serialize};

/// Floor arbitration mode (modules/01 §2).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorMode {
    /// One speaker at a time.
    Exclusive,
    /// Everyone may speak; floor control degenerates to mute governance.
    Open,
    /// Up to `FloorLimits` simultaneous speakers with queueing above cap.
    Hybrid,
}

/// Why a floor request was denied (modules/01 §3 — closed on purpose:
/// unknown `reason` strings must fail deserialization in PoC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenyReason {
    /// Queue at ceiling.
    Busy,
    /// Would exceed the mode's simultaneous-speaker cap.
    ExceedsCeiling,
    /// Peer not a room member.
    NotMember,
    /// Request rate guard tripped.
    RateLimited,
    /// Preempt attempt without strictly-higher priority.
    PreemptPriority,
    /// Peer has no live media transport.
    NoMedia,
}

/// Dispatch/Field role split (modules/01 §1): dispatch sees the queue, field does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Dispatch,
    Field,
}

/// Per-peer roster entry carried in `PeerList` / snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub role: Role,
    /// Unix millis at join (roster display only — never used for ordering).
    pub connected_since_ms: u64,
}

/// Queue entry (dispatch-scope only, D12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub peer: PeerId,
    pub priority: u8,
}

/// WebRTC transport parameters handed to a client (modules/02 `TransportInfo`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportInfo {
    pub ice: serde_json::Value,
    pub dtls: serde_json::Value,
    pub addrs: serde_json::Value,
}

/// Role-scoped snapshot payload (D12/D16): field peers never see the queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "payload", rename_all = "snake_case")]
pub enum ServerSnapshotPayload {
    Field(FieldSnapshot),
    Dispatch(DispatchSnapshot),
}

/// Field-scope snapshot: grants + mute state, NO queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSnapshot {
    pub mode: FloorMode,
    pub grants: Vec<PeerId>,
    pub muted: Vec<PeerId>,
    pub generation: u64,
    pub peers: Vec<PeerInfo>,
}

/// Dispatch-scope snapshot: adds the pending queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchSnapshot {
    #[serde(flatten)]
    pub field: FieldSnapshot,
    pub queue: Vec<Pending>,
}

/// The signaling protocol message set (modules/02 table + D16 additions).
///
/// Wire rules (modules/02 §4):
/// - `#[serde(tag = "type", rename_all = "snake_case")]` on every enum;
/// - field names snake_case on the wire (TS unions mirror exactly);
/// - additive-only evolution: `Join.v` protocol version, echoed in `Welcome`;
///   undeserializable versioned payloads → `Error { code: "bad_version" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    // ── client → server ────────────────────────────────────────────────
    /// Join with JWT; `v` = protocol version this client speaks (D16).
    Join {
        v: u32,
        jwt: String,
    },
    FloorRequest {
        priority: u8,
        preempt: bool,
    },
    FloorRelease,
    ModeChange {
        mode: FloorMode,
    },
    MuteSet {
        peer: PeerId,
        on: bool,
    },
    TransportCreate,
    TransportConnect {
        dtls: serde_json::Value,
    },
    Produce {
        rtp_parameters: serde_json::Value,
    },
    Consume {
        producer_id: serde_json::Value,
    },
    /// Media-plane resync request (modules/04 R1 snapshot-resync).
    Resync,

    // ── server → client ────────────────────────────────────────────────
    /// Join ack; echoes the negotiated protocol version (D16).
    Welcome {
        v: u32,
        peer_id: PeerId,
        turn_creds: serde_json::Value,
    },
    RouterCaps {
        media_codecs: serde_json::Value,
    },
    /// Full roster.
    PeerList {
        peers: Vec<PeerInfo>,
        /// Room generation at send (D16: receivers drop stale roster views).
        generation: u64,
    },
    /// Incremental roster delta (D16).
    PeerJoined {
        info: PeerInfo,
        generation: u64,
    },
    /// Incremental roster delta (D16).
    PeerLeft {
        peer: PeerId,
        generation: u64,
    },
    FloorGranted {
        grants: Vec<PeerId>,
        generation: u64,
    },
    FloorTaken {
        by: PeerId,
        generation: u64,
    },
    FloorDenied {
        reason: DenyReason,
        generation: u64,
    },
    FloorQueued {
        position: u32,
        generation: u64,
    },
    FloorIdle {
        generation: u64,
        reason: Option<String>,
    },
    TransportInfo {
        ice: serde_json::Value,
        dtls: serde_json::Value,
        addrs: serde_json::Value,
    },
    ProduceOk {
        producer_id: serde_json::Value,
    },
    ConsumeOk {
        producer_id: serde_json::Value,
        /// Server-created consumer id — mediasoup-client's local
        /// `transport.consume()` needs it (additive evolution, modules/02 §4).
        consumer_id: serde_json::Value,
        rtp_parameters: serde_json::Value,
    },
    /// SFU recovery completed (modules/04 R1).
    MediaRestart {
        room: RoomId,
        reason: String,
    },
    /// A peer's media plane died (modules/05 E11).
    MediaFailed {
        peer: PeerId,
    },
    /// Additive (PoC): a peer's producer is available for consumption —
    /// listeners open their downlink against this id (pull model).
    ProducerAvailable {
        peer: PeerId,
        producer_id: serde_json::Value,
    },
    /// Proactive token-renewal push (modules/02 §1; timer lives server-side).
    TokenRefresh {
        jwt: String,
    },
    /// Full state snapshot (request via `Resync`, or push after SFU recovery).
    ServerSnapshot {
        /// Snapshot mode: `field` or `dispatch` payload shape (D12 role-scoping).
        payload: ServerSnapshotPayload,
        generation: u64,
    },
    Error {
        code: String,
        detail: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PeerId;

    #[test]
    fn snake_case_tags() {
        let m = SignalingMessage::FloorGranted {
            grants: vec![PeerId::from("a")],
            generation: 42,
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["type"], "floor_granted");
        assert_eq!(v["generation"], 42);
    }

    #[test]
    fn join_carries_protocol_version() {
        let m = SignalingMessage::Join {
            v: 1,
            jwt: "tok".into(),
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["type"], "join");
        assert_eq!(v["v"], 1);
    }

    #[test]
    fn closed_deny_reason_rejects_unknown() {
        let raw = serde_json::json!({"type":"floor_denied","reason":"wat","generation":1});
        assert!(serde_json::from_value::<SignalingMessage>(raw).is_err());
    }
}
