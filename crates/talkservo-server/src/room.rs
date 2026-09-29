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
        for (_peer, m) in &self.members {
            if let Some(s) = skip {
                if *s == m.info.id {
                    continue;
                }
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
        let _ = peer_unused();
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
    pub fn handle_floor_message(&mut self, from: &PeerId, msg: &SignalingMessage) -> Vec<SignalingMessage> {
        let mut direct = Vec::new();
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
                    return direct;
                }
                self.last_request.insert(from.clone(), Instant::now());
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
            for m in emitted {
                self.broadcast(&m);
            }
        }
        direct
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
        Ok(())
    }

    /// Leave: emit PeerLeft delta to survivors + queue purge is core's job
    /// via Leave event; returns the survivor-facing delta.
    pub fn leave(&mut self, peer: &PeerId) -> Option<SignalingMessage> {
        if self.members.remove(peer).is_none() {
            return None;
        }
        let gen_before = self.floor.generation();
        let (next, _emitted) = self.floor.apply(
            &FloorEvent::Leave { peer: peer.clone() },
            &self.limits,
        );
        self.floor = next;
        self.last_request.remove(peer);
        let _ = gen_before;
        Some(SignalingMessage::PeerLeft {
            peer: peer.clone(),
            generation: self.floor.generation(),
        })
    }
}

fn peer_unused() {}
