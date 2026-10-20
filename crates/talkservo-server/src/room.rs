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
    /// actor->connection control channel (RoomCommand::Left = server-initiated
    /// close; the ws task answers with a 1000 Close frame - S2 eviction rides it)
    pub wire_tx: tokio::sync::mpsc::Sender<crate::ws::RoomCommand>,
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
    /// last E11 watchdog probe result per granted peer (plan-5 S1 admin
    /// `speaking` bit — cached here so AdminInspect never touches the SFU)
    last_activity: HashMap<PeerId, bool>,
    /// FloorRequest → request instant; consumed by the grant path to emit
    /// `grant_latency_ms` (modules/05 §Obs, acceptance #10). Kept across
    /// queueing so queued→promoted grants measure from the ORIGINAL request.
    pending_requests: HashMap<PeerId, Instant>,
    /// audit writer (set once by the actor; None in unit tests / audit-off runs)
    audit: Option<crate::audit::AuditTx>,
}

impl RoomState {
    pub fn new(
        id: RoomId,
        mode: talkservo_core::wire::FloorMode,
        max_peers: usize,
        max_queue: usize,
    ) -> Self {
        Self {
            id,
            members: HashMap::new(),
            floor: talkservo_core::floor::FloorState::initial(mode), // S2: registry default (was hard-coded Hybrid)
            limits: FloorLimits {
                max_peers,
                max_queue,
            },
            last_request: HashMap::new(),
            empty_since: None, // a new room has its creator en route
            pending_requests: HashMap::new(),
            producers_by_peer: HashMap::new(),
            last_activity: HashMap::new(),
            audit: None,
        }
    }

    pub fn set_audit(&mut self, tx: Option<crate::audit::AuditTx>) {
        self.audit = tx;
    }

    fn relay(&self, ev: crate::audit::AuditEvent) {
        if let Some(a) = &self.audit {
            a.emit(ev.room(self.id.0.as_ref()));
        }
    }

    pub fn relay_join(&self, peer: &PeerId) {
        self.relay(
            crate::audit::AuditEvent::new(crate::audit::AuditKind::Join).actor(peer.to_string()),
        );
    }

    pub fn relay_leave(&self, peer: &PeerId) {
        self.relay(
            crate::audit::AuditEvent::new(crate::audit::AuditKind::Leave).actor(peer.to_string()),
        );
    }

    /// timer/media paths: coarse relay of emitted transitions (no actor known)
    fn relay_floor_msgs(&self, msgs: &[SignalingMessage]) {
        if self.audit.is_none() {
            return;
        }
        for m in msgs {
            let rec = match m {
                SignalingMessage::FloorIdle { reason, .. } => Some(
                    crate::audit::AuditEvent::new(crate::audit::AuditKind::FloorIdle)
                        .reason(format!("{reason:?}")),
                ),
                SignalingMessage::FloorGranted { .. } => Some(crate::audit::AuditEvent::new(
                    crate::audit::AuditKind::FloorGrant,
                )),
                SignalingMessage::FloorTaken { .. } => Some(crate::audit::AuditEvent::new(
                    crate::audit::AuditKind::FloorTaken,
                )),
                _ => None,
            };
            if let Some(r) = rec {
                self.relay(r);
            }
        }
    }

    /// S3: Some((reason, target)) when the command is not authorized for `from`.
    /// Rule 10 (modules/01): ModeChange / MuteSet are AUTHORIZED commands - sender
    /// must hold Role::Dispatch. MuteSet additionally cannot target a Dispatch
    /// peer (dispatch-always-outranks; no mutual-fight path). Everything else
    /// (FloorRequest/Release by any peer) needs no grant.
    fn authorize_command(&self, from: &PeerId, msg: &SignalingMessage) -> Option<(String, String)> {
        let member = self.members.get(from)?;
        if member.info.role == Role::Dispatch {
            return None; // dispatch governs all (including other dispatch peers)
        }
        // S4: moderator claims OR into the grant (R3/R4: per-room authority;
        // claims ride the token and were copied onto the member at Join).
        if member
            .info
            .mod_rooms
            .iter()
            .any(|m| m == self.id.0.as_ref())
        {
            return None; // room moderator governs THIS room (Dispatch targets
            // stay unmutable - the target check below still runs)
        }
        match msg {
            SignalingMessage::ModeChange { .. } => {
                Some(("mode_change_requires_dispatch".into(), String::new()))
            }
            SignalingMessage::MuteSet { peer, .. } => {
                let target_role = self.members.get(peer).map(|m| m.info.role);
                match target_role {
                    Some(Role::Dispatch) => Some(("cannot_mute_dispatch".into(), peer.to_string())),
                    _ => Some(("mute_requires_dispatch".into(), peer.to_string())),
                }
            }
            _ => None,
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
                (Role::Field, SignalingMessage::ServerSnapshot { payload, .. }) => {
                    matches!(payload, ServerSnapshotPayload::Field(_))
                }
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
            Role::Dispatch => {
                ServerSnapshotPayload::Dispatch(talkservo_core::wire::DispatchSnapshot {
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
                        muted: self.floor.muted().to_vec(),
                        generation,
                        peers,
                    },
                })
            }
            Role::Field => ServerSnapshotPayload::Field(talkservo_core::wire::FieldSnapshot {
                mode: self.floor.mode(),
                grants: self.floor.grants().to_vec(),
                muted: self.floor.muted().to_vec(),
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

        // S3 command authorization (plan-7 R3 ruling: Dispatch always outranks;
        // mod-claims ride S4 and will OR into this check). Checks live HERE, not
        // in core: core stays a pure machine (D5/I-O-at-the-edge); denial never
        // reaches apply(), so the state machine stays serializable.
        if let Some(deny) = self.authorize_command(from, msg) {
            let generation = self.floor.generation();
            self.relay(
                crate::audit::AuditEvent::new(crate::audit::AuditKind::AuthzDenied)
                    .actor(from.to_string())
                    .target(deny.1.clone())
                    .reason(deny.0.clone()),
            );
            direct.push(SignalingMessage::FloorDenied {
                reason: talkservo_core::wire::DenyReason::Unauthorized,
                generation,
            });
            return FloorOutcome {
                direct,
                mutated: false,
            };
        }

        let ev = match msg {
            SignalingMessage::FloorRequest { priority, preempt } => {
                if !cooldown_ok {
                    self.last_request.insert(from.clone(), Instant::now());
                    let generation = self.floor.generation();
                    direct.push(SignalingMessage::FloorDenied {
                        reason: talkservo_core::wire::DenyReason::RateLimited,
                        generation,
                    });
                    return FloorOutcome {
                        direct,
                        mutated: false,
                    };
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
            let pre_mode = self.floor.mode();
            let pre_grants: Vec<String> =
                self.floor.grants().iter().map(|g| g.to_string()).collect();
            let pre_muted: Vec<String> = self.floor.muted().iter().map(|m| m.to_string()).collect();
            let (next, emitted) = self.floor.apply(&ev, &self.limits);
            self.floor = next;
            mutated = true; // any apply() result reflects a floor transition

            if self.audit.is_some() {
                match &ev {
                    FloorEvent::ModeChange { mode, actor } => self.relay(
                        crate::audit::AuditEvent::new(crate::audit::AuditKind::ModeChange)
                            .actor(actor.to_string())
                            .change(
                                "mode",
                                format!("{pre_mode:?}").into(),
                                format!("{mode:?}").into(),
                            ),
                    ),
                    FloorEvent::MuteSet { actor, peer, on } => self.relay(
                        crate::audit::AuditEvent::new(crate::audit::AuditKind::MuteSet)
                            .actor(actor.to_string())
                            .target(peer.to_string())
                            .change(
                                "muted",
                                pre_muted.contains(&peer.to_string()).into(),
                                (*on).into(),
                            ),
                    ),
                    _ => {}
                }
                for m in &emitted {
                    let rec = match m {
                        SignalingMessage::FloorGranted { grants, .. } => Some(
                            crate::audit::AuditEvent::new(crate::audit::AuditKind::FloorGrant)
                                .actor(from.to_string())
                                .change(
                                    "grants",
                                    pre_grants.clone().into(),
                                    grants
                                        .iter()
                                        .map(|g| g.to_string())
                                        .collect::<Vec<_>>()
                                        .into(),
                                ),
                        ),
                        SignalingMessage::FloorTaken { by, .. } => Some(
                            crate::audit::AuditEvent::new(crate::audit::AuditKind::FloorTaken)
                                .actor(by.to_string()),
                        ),
                        SignalingMessage::FloorIdle { reason, .. } => Some(
                            crate::audit::AuditEvent::new(crate::audit::AuditKind::FloorIdle)
                                .reason(format!("{reason:?}")),
                        ),
                        _ => None,
                    };
                    if let Some(r) = rec {
                        self.relay(r);
                    }
                }
            }
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
                .any(|(peer, pid)| pid == producer_id && self.floor.grants().contains(peer))
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

    /// watchdog writes the probe result (plan-5 S1)
    pub fn note_activity(&mut self, peer: &PeerId, active: bool) {
        self.last_activity.insert(peer.clone(), active);
    }

    /// Read-only admin snapshot (plan-5 S1): RoomState only, no SFU calls.
    pub fn admin_report(&self) -> crate::ws::RoomReport {
        let holders: Vec<String> = self
            .floor
            .grants()
            .iter()
            .map(|p| p.0.to_string())
            .collect();
        let holder_ids: Vec<PeerId> = self.floor.grants().to_vec();
        let speaking: Vec<String> = holder_ids
            .iter()
            .filter(|p| *self.last_activity.get(*p).unwrap_or(&false))
            .map(|p| p.0.to_string())
            .collect();
        crate::ws::RoomReport {
            room: self.id.0.to_string(),
            mode: self.floor.mode(),
            holders,
            queue_len: self.floor.queue().len(),
            peers: self
                .members
                .values()
                .map(|m| crate::ws::PeerEntry {
                    id: m.info.id.0.to_string(),
                    role: m.info.role,
                    mod_rooms: m.info.mod_rooms.clone(),
                    granted: self.floor.is_granted(&m.info.id),
                    muted: self.floor.is_muted(&m.info.id),
                    connected_since_ms: m.info.connected_since_ms,
                })
                .collect(),
            muted: self.floor.muted().iter().map(|p| p.0.to_string()).collect(),
            speaking,
        }
    }

    /// Immutable view for the media reconciliation call (D13).
    pub fn floor_ref(&self) -> &talkservo_core::floor::FloorState {
        &self.floor
    }

    /// E11 (F-sequence): apply MediaDown for a granted peer whose grace
    /// elapsed — core releases + promotes; broadcast handled by caller.
    pub fn apply_media_down(&mut self, peer: &PeerId) -> Vec<SignalingMessage> {
        let (next, emitted) = self
            .floor
            .apply(&FloorEvent::MediaDown { peer: peer.clone() }, &self.limits);
        self.floor = next;
        self.relay_floor_msgs(&emitted);
        emitted
    }

    /// R7 max-hold timer: apply Timeout (core drops grants + promotes).
    pub fn apply_timeout(&mut self) -> Vec<SignalingMessage> {
        let (next, emitted) = self.floor.apply(&FloorEvent::Timeout, &self.limits);
        self.floor = next;
        self.relay_floor_msgs(&emitted);
        emitted
    }

    /// Dispatch-config flag: max-hold applies only when configured (open mode
    /// runs with FLOOR_MAX_HOLD_MS=off — modules/06).
    pub fn max_hold_armed(&self) -> bool {
        self.limits.max_peers > 0
    }

    /// Join a validated identity. `Err(code)` = AlreadyJoined.
    pub fn join(
        &mut self,
        peer: PeerId,
        role: Role,
        sink: Sink,
        wire_tx: tokio::sync::mpsc::Sender<crate::ws::RoomCommand>,
        mod_rooms: Vec<String>,
        since_ms: u64,
    ) -> Result<(), &'static str> {
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
                    mod_rooms,
                },
                sink,
                wire_tx,
            },
        );
        self.empty_since = None; // occupied again — reaper stands down
        Ok(())
    }

    /// Leave: emit PeerLeft delta to survivors + queue purge is core's job
    /// via Leave event; returns the survivor-facing delta.
    /// Operator-initiated removal (S6 kick / S2 evict): same teardown, the
    /// survivor delta carries kind=removed (R2: no reason/actor on the wire).
    pub fn leave_removed(&mut self, peer: &PeerId) -> Option<SignalingMessage> {
        let delta = self.leave(peer)?;
        match delta {
            SignalingMessage::PeerLeft {
                generation, peer, ..
            } => Some(SignalingMessage::PeerLeft {
                peer,
                generation,
                kind: talkservo_core::wire::LeaveKind::Removed,
            }),
            other => Some(other),
        }
    }

    pub fn leave(&mut self, peer: &PeerId) -> Option<SignalingMessage> {
        self.members.remove(peer)?;
        let gen_before = self.floor.generation();
        let (next, _emitted) = self
            .floor
            .apply(&FloorEvent::Leave { peer: peer.clone() }, &self.limits);
        self.floor = next;
        self.last_request.remove(peer);
        self.pending_requests.remove(peer);
        let _ = gen_before;
        Some(SignalingMessage::PeerLeft {
            peer: peer.clone(),
            generation: self.floor.generation(),
            kind: talkservo_core::wire::LeaveKind::Left,
        })
    }
}
