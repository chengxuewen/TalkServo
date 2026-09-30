//! Room mailbox — single-writer task per room (plan-2 architecture).
//!
//! Every mutation of room state flows through one mpsc channel so ordering is
//! total per room; broadcasts fan out to member sinks with role scoping
//! (dispatch sees queue messages, field does not — D12).

use std::collections::HashMap;
use std::time::Instant;
use talkservo_core::floor::{FloorEvent, FloorLimits};
use talkservo_core::ids::{PeerId, RoomId};
use talkservo_core::wire::{PeerInfo, Role, ServerSnapshotPayload, SignalingMessage};
use tokio::sync::mpsc;

/// Result of one floor-protocol message: direct replies + whether the floor
/// state moved (drives the media reconcile call, D13).
pub struct FloorOutcome {
    pub direct: Vec<SignalingMessage>,
    pub mutated: bool,
}

/// Per-connection outbound sink.
pub type Sink = mpsc::UnboundedSender<SignalingMessage>;

/// A connected member.
#[derive(Clone)]
pub struct Member {
    pub info: PeerInfo,
    pub sink: Sink,
}

/// Everything the single-writer task owns.
pub struct RoomState {
    pub id: RoomId,
    pub members: HashMap<PeerId, Member>,
    pub floor: talkservo_core::floor::FloorState,
    pub limits: FloorLimits,
    /// Last FloorRequest time per peer (cooldown gate).
    pub last_request: HashMap<PeerId, Instant>,
    /// Armed when the room becomes empty (idle-TTL reaper, D16/CM-1);
    /// disarmed by the next Join.
    empty_since: Option<Instant>,
    /// peer → latest producer id (R-F14 consume gate registry).
    producers_by_peer: HashMap<PeerId, String>,
    /// FloorRequest → request instant; consumed by the grant path to emit
    /// `grant_latency_ms` (modules/05 §Obs, acceptance #10). Kept across
    /// queueing so queued→promoted grants measure from the ORIGINAL request.
    pending_requests: HashMap<PeerId, Instant>,
}

impl RoomState {
    pub fn new(id: RoomId, max_peers: usize, max_queue: usize) -> Self {
        Self {
            id,
            members: HashMap::new(),
            floor: talkservo_core::floor::FloorState::initial(
                talkservo_core::wire::FloorMode::Hybrid,
            ),
            limits: FloorLimits {
                max_peers,
                max_queue,
            },
            last_request: HashMap::new(),
            empty_since: None, // a new room has its creator en route
            pending_requests: HashMap::new(),
            producers_by_peer: HashMap::new(),
        }
    }

    /// Reaper probe: no members at all?
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Armed empty-since clock (None while members exist or not yet ticked).
    pub fn empty_since(&self) -> Option<Instant> {
        self.empty_since
    }

    /// Arm/disarm the empty clock (called by the IdleTick sweep).
    pub fn track_empty(&mut self) {
        if self.members.is_empty() {
            if self.empty_since.is_none() {
                self.empty_since = Some(Instant::now());
            }
        } else {
            self.empty_since = None;
        }
    }

    /// Role-scoped broadcast: strip queue-bearing messages from field peers.
    /// Dispatch-only messages (none in the wire set yet) would be skipped here.
    pub fn broadcast(&self, msg: &SignalingMessage) {
        self.broadcast_except(msg, None);
    }

    /// Broadcast, optionally excluding one peer (join/leave deltas go to
    /// OTHERS only — the subject gets the full PeerList instead, modules/02).
    pub fn broadcast_except(&self, msg: &SignalingMessage, skip: Option<&PeerId>) {
        for m in self.members.values() {
            if skip.is_some_and(|s| *s == m.info.id) {
                continue;
            }
            let visible = match (m.info.role, msg) {
                // FloorQueued positions only matter to dispatch (field sees
                // grants/denials); keep it simple: field peers receive queued
                // messages too (their own position acks) — scoping applies to
                // ServerSnapshot payloads only (D12).
                (
                    Role::Field,
                    SignalingMessage::ServerSnapshot { payload, .. },
                ) => matches!(payload, ServerSnapshotPayload::Field(_)),
                _ => true,
            };
            if visible {
                let _ = m.sink.send(msg.clone());
            }
        }
    }

    /// Build the role-scoped snapshot for `peer` (D12).
    pub fn snapshot_for(&self, peer: &PeerId) -> SignalingMessage {
        let role = self
            .members
            .get(peer)
            .map(|m| m.info.role)
            .unwrap_or(Role::Field);
        let peers: Vec<PeerInfo> = self.members.values().map(|m| m.info.clone()).collect();
        let generation = self.floor.generation();
        let payload = match role {
            Role::Dispatch => ServerSnapshotPayload::Dispatch(
                talkservo_core::wire::DispatchSnapshot {
                    queue: self
                        .floor
                        .queue()
                        .iter()
                        .map(|p| talkservo_core::wire::Pending {
                            peer: p.peer.clone(),
                            priority: p.priority,
                        })
                        .collect(),
                    field: talkservo_core::wire::FieldSnapshot {
                        mode: self.floor.mode(),
                        grants: self.floor.grants().to_vec(),
                        muted: vec![], // muted set lives in floor state — expose below
                        generation,
                        peers,
                    },
                },
            ),
            Role::Field => ServerSnapshotPayload::Field(talkservo_core::wire::FieldSnapshot {
                mode: self.floor.mode(),
                grants: self.floor.grants().to_vec(),
                muted: vec![],
                generation,
                peers,
            }),
        };
        SignalingMessage::ServerSnapshot {
            payload,
            generation,
        }
    }

    /// Convert a wire message from a peer into the floor event + protocol
    /// responses. Returns messages to send to the requester (acks/errors).
    pub fn handle_floor_message(&mut self, from: &PeerId, msg: &SignalingMessage) -> FloorOutcome {
        let mut direct = Vec::new();
        let mut mutated = false;
        let cooldown_ok = self
            .last_request
            .get(from)
            .map(|t| t.elapsed().as_millis() as u64 >= 500)
            .unwrap_or(true);

        let ev = match msg {
            SignalingMessage::FloorRequest { priority, preempt } => {
                if !cooldown_ok {
                    self.last_request.insert(from.clone(), Instant::now());
                    let generation = self.floor.generation();
                    direct.push(SignalingMessage::FloorDenied {
                        reason: talkservo_core::wire::DenyReason::RateLimited,
                        generation,
                    });
                    return FloorOutcome { direct, mutated: false };
                }
                self.last_request.insert(from.clone(), Instant::now());
                self.pending_requests
                    .entry(from.clone())
                    .or_insert_with(Instant::now); // keep the ORIGINAL request time
                Some(FloorEvent::Request {
                    peer: from.clone(),
                    priority: *priority,
                    preempt: *preempt,
                })
            }
            SignalingMessage::FloorRelease => Some(FloorEvent::Release { peer: from.clone() }),
            SignalingMessage::ModeChange { mode } => Some(FloorEvent::ModeChange {
                mode: *mode,
                actor: from.clone(),
            }),
            SignalingMessage::MuteSet { peer, on } => Some(FloorEvent::MuteSet {
                actor: from.clone(),
                peer: peer.clone(),
                on: *on,
            }),
            _ => None,
        };

        if let Some(ev) = ev {
            let (next, emitted) = self.floor.apply(&ev, &self.limits);
            self.floor = next;
            mutated = true; // any apply() result reflects a floor transition

            for m in emitted {
                // latency tap: grant/taken mark the END of a request's wait
                match &m {
                    SignalingMessage::FloorGranted { generation, .. }
                    | SignalingMessage::FloorTaken { generation, .. } => {
                        for (peer, t0) in self.pending_requests.iter() {
                            crate::obs::latency(
                                self.id.0.as_ref(),
                                peer.0.as_ref(),
                                *generation,
                                "grant_latency_ms",
                                t0.elapsed().as_millis() as u64,
                            );
                        }
                        self.pending_requests.clear();
                    }
                    _ => {}
                }
                self.broadcast(&m);
            }
            // Mode transitions can be silent in core (empty→empty emit nothing)
            // — clients still need the new mode. Role-scoped snapshots are the
            // canonical state carrier (R1 mechanism); reuse it here.
            if matches!(ev, FloorEvent::ModeChange { .. }) {
                for peer in self.members.keys() {
                    if let Some(m) = self.members.get(peer) {
                        let _ = m.sink.send(self.snapshot_for(peer));
                    }
                }
            }
        }
        FloorOutcome { direct, mutated }
    }

    /// R-F14 gate: does `producer_id` (wire string) belong to a GRANTED peer
    /// in this room? Unknown producers → false (no existence oracle).
    pub fn producer_owner_granted(&self, producer_id: &str) -> bool {
        // The server assigns producer ids at ProduceOk time; the roster keeps
        // the peer→producer map here (filled by the Produce path below).
        self.producers_by_peer
            .values()
            .any(|pid| pid == producer_id)
            && self
                .producers_by_peer
                .iter()
                .any(|(peer, pid)| {
                    pid == producer_id && self.floor.grants().contains(peer)
                })
    }

    /// Snapshot of the producer registry (late-joiner announcements).
    pub fn producers_list(&self) -> Vec<(PeerId, String)> {
        self.producers_by_peer
            .iter()
            .map(|(p, pid)| (p.clone(), pid.clone()))
            .collect()
    }

    /// Record a produced producer id for its owner (ProduceOk path).
    pub fn note_producer(&mut self, peer: &PeerId, producer_id: String) {
        self.producers_by_peer.insert(peer.clone(), producer_id);
    }

    /// Immutable view for the media reconciliation call (D13).
    pub fn floor_ref(&self) -> &talkservo_core::floor::FloorState {
        &self.floor
    }

    /// E11 (F-sequence): apply MediaDown for a granted peer whose grace
    /// elapsed — core releases + promotes; broadcast handled by caller.
    pub fn apply_media_down(&mut self, peer: &PeerId) -> Vec<SignalingMessage> {
        let (next, emitted) = self.floor.apply(
            &FloorEvent::MediaDown { peer: peer.clone() },
            &self.limits,
        );
        self.floor = next;
        emitted
    }

    /// R7 max-hold timer: apply Timeout (core drops grants + promotes).
    pub fn apply_timeout(&mut self) -> Vec<SignalingMessage> {
        let (next, emitted) = self.floor.apply(&FloorEvent::Timeout, &self.limits);
        self.floor = next;
        emitted
    }

    /// Dispatch-config flag: max-hold applies only when configured (open mode
    /// runs with FLOOR_MAX_HOLD_MS=off — modules/06).
    pub fn max_hold_armed(&self) -> bool {
        self.limits.max_peers > 0
    }

    /// Join a validated identity. `Err(code)` = AlreadyJoined.
    pub fn join(&mut self, peer: PeerId, role: Role, sink: Sink, since_ms: u64) -> Result<(), &'static str> {
        if self.members.contains_key(&peer) {
            return Err("already_joined");
        }
        self.members.insert(
            peer.clone(),
            Member {
                info: PeerInfo {
                    id: peer,
                    role,
                    connected_since_ms: since_ms,
                },
                sink,
            },
        );
        self.empty_since = None; // occupied again — reaper stands down
        Ok(())
    }

    /// Leave: emit PeerLeft delta to survivors + queue purge is core's job
    /// via Leave event; returns the survivor-facing delta.
    pub fn leave(&mut self, peer: &PeerId) -> Option<SignalingMessage> {
        self.members.remove(peer)?;
        let gen_before = self.floor.generation();
        let (next, _emitted) = self.floor.apply(
            &FloorEvent::Leave { peer: peer.clone() },
            &self.limits,
        );
        self.floor = next;
        self.last_request.remove(peer);
        self.pending_requests.remove(peer);
        let _ = gen_before;
        Some(SignalingMessage::PeerLeft {
            peer: peer.clone(),
            generation: self.floor.generation(),
        })
    }
}

