//! FloorState — the pure floor-arbitration reducer (modules/01 rules 1-9, D12 v2 model).
//!
//! Total function design: `apply()` takes `(state, event, limits)` and returns
//! `(new_state, emitted_wire_messages)`. No I/O, no clocks, no randomness inside
//! the reducer — `generation` advances as `prev + 1` (fresh entropy enters only
//! at `initial()` via SystemTime nanos; tests pin determinism via
//! `initial_with_generation`).

use crate::ids::PeerId;
use crate::wire::{DenyReason, SignalingMessage};
use std::time::{SystemTime, UNIX_EPOCH};

/// Simultaneous-speaker and queue caps, passed per-call so the core stays
/// config-free (values come from server config; modules/06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorLimits {
    /// Max simultaneous granted speakers (Exclusive=1, Hybrid=2 default shape).
    pub max_peers: usize,
    /// Max queued requests before `Denied{Busy}`.
    pub max_queue: usize,
}

impl Default for FloorLimits {
    fn default() -> Self {
        // modules/01 defaults: exclusive=1/hybrid=2 grants; queue depth 16 at PoC
        Self {
            max_peers: 1,
            max_queue: 16,
        }
    }
}

/// One waiting floor request (dispatch-scope only, D12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub peer: PeerId,
    pub priority: u8,
}

/// Immutable floor state — every `apply()` returns a fresh value (no mutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorState {
    mode: crate::wire::FloorMode,
    grants: Vec<PeerId>,
    /// Request-priority of each grant, index-aligned with `grants`
    /// (modules/01 §3: preempt compares against the sitting holder's
    /// request priority — the state must carry it).
    grant_priority: Vec<u8>,
    muted: Vec<PeerId>,
    queue: Vec<Pending>,
    generation: u64,
}

pub use crate::wire::FloorMode;

/// Events the reducer consumes (server-derived; modules/01 §4).
/// `actor` fields mark who drove the change (modules/01 normative signature,
/// D16 audit surface) — the core does not police authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloorEvent {
    Request {
        peer: PeerId,
        priority: u8,
        preempt: bool,
    },
    Release {
        peer: PeerId,
    },
    Leave {
        peer: PeerId,
    },
    MediaDown {
        peer: PeerId,
    },
    Timeout,
    ModeChange {
        mode: FloorMode,
        actor: PeerId,
    },
    MuteSet {
        actor: PeerId,
        peer: PeerId,
        on: bool,
    },
}

impl FloorState {
    /// Production constructor: generation seeded from SystemTime nanos
    /// (nonzero, high-resolution — two calls never collide in practice).
    pub fn initial(mode: FloorMode) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self::initial_with_generation(mode, nanos.max(1))
    }

    /// Deterministic constructor for tests and replay: any generation is legal
    /// (P1-M review: makes the coverage gate deterministic; ordering assumptions
    /// on generation values are forbidden — comparison only ever checks equality).
    pub fn initial_with_generation(mode: FloorMode, generation: u64) -> Self {
        Self {
            mode,
            grants: Vec::new(),
            grant_priority: Vec::new(),
            muted: Vec::new(),
            queue: Vec::new(),
            generation,
        }
    }

    // ── accessors ───────────────────────────────────────────────────────
    pub fn mode(&self) -> FloorMode {
        self.mode
    }
    pub fn grants(&self) -> &[PeerId] {
        &self.grants
    }
    pub fn queue(&self) -> &[Pending] {
        &self.queue
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn is_granted(&self, peer: &PeerId) -> bool {
        self.grants.contains(peer)
    }
    pub fn is_muted(&self, peer: &PeerId) -> bool {
        self.muted.contains(peer)
    }
    pub fn queue_position(&self, peer: &PeerId) -> Option<usize> {
        self.queue.iter().position(|p| &p.peer == peer)
    }

    fn capacity(&self) -> usize {
        match self.mode {
            FloorMode::Exclusive => 1,
            FloorMode::Hybrid => 2,
            FloorMode::Open => usize::MAX,
        }
    }

    /// The reducer (modules/01 rules 1-9). Pure: same inputs → same outputs.
    pub fn apply(
        &self,
        ev: &FloorEvent,
        limits: &FloorLimits,
    ) -> (FloorState, Vec<SignalingMessage>) {
        let mut next = self.clone();
        let mut out = Vec::new();

        match ev {
            // ── R1/R3/R4/R5: Request ────────────────────────────────────
            FloorEvent::Request {
                peer,
                priority,
                preempt,
            } => {
                if self.is_granted(peer) || self.queue_position(peer).is_some() {
                    // R4 idempotent: echo current position, no state change
                    if let Some(pos) = self.queue_position(peer) {
                        out.push(SignalingMessage::FloorQueued {
                            position: pos as u32,
                            generation: self.generation,
                        });
                    }
                    return (next, out);
                }

                // R5 preempt: strictly-higher priority may displace the sole
                // exclusive holder (modules/01: preempt is an Exclusive-mode
                // instrument; other modes ignore the flag — space path below)
                if *preempt && self.mode == FloorMode::Exclusive && !self.grants.is_empty() {
                    let holder_priority = self.grant_priority[0];
                    if *priority > holder_priority {
                        next.grants = vec![peer.clone()];
                        next.grant_priority = vec![*priority];
                        next.generation += 1;
                        // `by` = the new holder; this message routes to the
                        // displaced peer ("your floor was taken by …")
                        out.push(SignalingMessage::FloorTaken {
                            by: peer.clone(),
                            generation: next.generation,
                        });
                        out.push(SignalingMessage::FloorGranted {
                            grants: next.grants.clone(),
                            generation: next.generation,
                        });
                        return (next, out);
                    }
                    // equal/lower preempt → denied, no-op (R5)
                    out.push(SignalingMessage::FloorDenied {
                        reason: DenyReason::PreemptPriority,
                        generation: self.generation,
                    });
                    return (next, out);
                }

                // capacity check
                if self.grants.len() < self.capacity() {
                    next.grants.push(peer.clone());
                    next.grant_priority.push(*priority);
                    next.generation += 1;
                    out.push(SignalingMessage::FloorGranted {
                        grants: next.grants.clone(),
                        generation: next.generation,
                    });
                    return (next, out);
                }

                // queue path (R1/R3): insert priority-desc, FIFO within tier
                let entry = Pending {
                    peer: peer.clone(),
                    priority: *priority,
                };
                match next.queue.len().cmp(&limits.max_queue) {
                    std::cmp::Ordering::Greater => {
                        // over ceiling: deny Busy (R5-adjacent; D16 caps)
                        return (
                            next,
                            vec![SignalingMessage::FloorDenied {
                                reason: DenyReason::Busy,
                                generation: self.generation,
                            }],
                        );
                    }
                    std::cmp::Ordering::Equal => {
                        return (
                            next,
                            vec![SignalingMessage::FloorDenied {
                                reason: DenyReason::Busy,
                                generation: self.generation,
                            }],
                        );
                    }
                    std::cmp::Ordering::Less => {}
                }
                let insert_at = next
                    .queue
                    .iter()
                    .position(|p| p.priority < entry.priority)
                    .unwrap_or(next.queue.len());
                next.queue.insert(insert_at, entry);
                next.generation += 1;
                out.push(SignalingMessage::FloorQueued {
                    position: insert_at as u32,
                    generation: next.generation,
                });
                // R3: every PRE-EXISTING entry from insert_at on was shifted
                // down by one — rebroadcast their new positions (the newcomer
                // at insert_at already got its own message above)
                for (pos, p) in next.queue.iter().enumerate().skip(insert_at + 1) {
                    out.push(SignalingMessage::FloorQueued {
                        position: pos as u32,
                        generation: next.generation,
                    });
                    let _ = p;
                }
                (next, out)
            }

            // ── R2: Release → auto-promote ──────────────────────────────
            FloorEvent::Release { peer } => {
                if !self.is_granted(peer) {
                    return (next, out); // R8 no-op
                }
                let idx = self.grants.iter().position(|p| p == peer).unwrap();
                next.grants.remove(idx);
                next.grant_priority.remove(idx);
                next.generation += 1;
                Self::promote_or_idle(&mut next, &mut out);
                (next, out)
            }

            // ── Leave: release-like + D16 queue purge ───────────────────
            FloorEvent::Leave { peer } => {
                let was_granted = self.is_granted(peer);
                let was_queued_at = self.queue_position(peer);
                if !was_granted && was_queued_at.is_none() {
                    return (next, out); // R8 no-op
                }
                if let Some(idx) = self.grants.iter().position(|p| p == peer) {
                    next.grants.remove(idx);
                    next.grant_priority.remove(idx);
                }
                if let Some(pos) = was_queued_at {
                    next.queue.remove(pos);
                    next.generation += 1;
                    // D16: rebroadcast shifted positions; the leaver gets nothing
                    for (i, p) in next.queue.iter().enumerate().skip(pos) {
                        out.push(SignalingMessage::FloorQueued {
                            position: i as u32,
                            generation: next.generation,
                        });
                        let _ = p;
                    }
                    return (next, out);
                }
                next.generation += 1;
                Self::promote_or_idle(&mut next, &mut out);
                (next, out)
            }

            // ── R6: MediaDown = immediate release (grace is server-domain) ──
            FloorEvent::MediaDown { peer } => {
                let idx = match self.grants.iter().position(|p| p == peer) {
                    Some(i) => i,
                    None => return (next, out),
                };
                next.grants.remove(idx);
                next.grant_priority.remove(idx);
                next.generation += 1;
                Self::promote_or_idle(&mut next, &mut out);
                (next, out)
            }

            // ── R7: Timeout ─────────────────────────────────────────────
            FloorEvent::Timeout => {
                if self.grants.is_empty() {
                    return (next, out);
                }
                next.grants.clear();
                next.grant_priority.clear();
                next.generation += 1;
                out.push(SignalingMessage::FloorIdle {
                    generation: next.generation,
                    reason: Some("timeout".into()),
                });
                // timed-out holder freed the floor — promote like release;
                // the timeout idle IS the idle emission (no second one)
                if !next.queue.is_empty() {
                    let head = next.queue.remove(0);
                    next.grants.push(head.peer);
                    next.grant_priority.push(head.priority);
                    next.generation += 1;
                    out.push(SignalingMessage::FloorGranted {
                        grants: next.grants.clone(),
                        generation: next.generation,
                    });
                    for (i, _) in next.queue.iter().enumerate() {
                        out.push(SignalingMessage::FloorQueued {
                            position: i as u32,
                            generation: next.generation,
                        });
                    }
                }
                (next, out)
            }

            // ── R9: ModeChange ──────────────────────────────────────────
            FloorEvent::ModeChange { mode, actor } => {
                if *mode == self.mode {
                    return (next, out);
                }
                next.mode = *mode;
                next.generation += 1;
                let _ = actor;
                match (*mode, self.mode) {
                    // anything → Open: drop queue, everyone may speak
                    (FloorMode::Open, _) => {
                        next.queue.clear();
                        if !self.grants.is_empty() {
                            out.push(SignalingMessage::FloorIdle {
                                generation: next.generation,
                                reason: None,
                            });
                        }
                    }
                    // Open → tighter: keep first-cap grants, rest queue in order
                    (FloorMode::Exclusive, FloorMode::Open)
                    | (FloorMode::Hybrid, FloorMode::Open) => {
                        let cap = next.capacity();
                        if self.grants.len() > cap {
                            let demoted: Vec<PeerId> = self.grants[cap..].to_vec();
                            let demoted_prio: Vec<u8> = self.grant_priority[cap..].to_vec();
                            next.grants = self.grants[..cap].to_vec();
                            next.grant_priority = self.grant_priority[..cap].to_vec();
                            for (p, prio) in demoted.into_iter().zip(demoted_prio) {
                                next.queue.push(Pending {
                                    peer: p,
                                    priority: prio,
                                });
                            }
                            out.push(SignalingMessage::FloorQueued {
                                position: 0,
                                generation: next.generation,
                            });
                        }
                    }
                    (FloorMode::Exclusive, FloorMode::Hybrid) => {
                        // cap drops 2 → 1: demote the tail
                        let cap = next.capacity();
                        if self.grants.len() > cap {
                            let demoted: Vec<PeerId> = self.grants[cap..].to_vec();
                            next.grants = self.grants[..cap].to_vec();
                            next.grant_priority = self.grant_priority[..cap].to_vec();
                            for (i, p) in demoted.into_iter().enumerate() {
                                next.queue.push(Pending {
                                    peer: p,
                                    priority: self.grant_priority[cap + i],
                                });
                            }
                            out.push(SignalingMessage::FloorQueued {
                                position: 0,
                                generation: next.generation,
                            });
                        }
                    }
                    (FloorMode::Hybrid, FloorMode::Exclusive) => {
                        let cap = next.capacity();
                        if self.grants.len() > cap {
                            let demoted: Vec<PeerId> = self.grants[cap..].to_vec();
                            next.grants = self.grants[..cap].to_vec();
                            next.grant_priority = self.grant_priority[..cap].to_vec();
                            for (i, p) in demoted.into_iter().enumerate() {
                                next.queue.push(Pending {
                                    peer: p,
                                    priority: self.grant_priority[cap + i],
                                });
                            }
                            out.push(SignalingMessage::FloorQueued {
                                position: 0,
                                generation: next.generation,
                            });
                        }
                    }
                    _ => {}
                }
                (next, out)
            }

            // ── mute governance (open mode) ─────────────────────────────
            FloorEvent::MuteSet { actor, peer, on } => {
                let _ = actor;
                // R8: muting a peer the floor state doesn't know (not granted,
                // not queued) is a no-op — roster membership is server-domain
                let known = self.is_granted(peer) || self.queue_position(peer).is_some();
                if *on && known {
                    if !next.muted.contains(peer) {
                        next.muted.push(peer.clone());
                        next.generation += 1;
                    }
                } else if next.muted.contains(peer) {
                    next.muted.retain(|p| p != peer);
                    next.generation += 1;
                }
                (next, out)
            }
        }
    }

    /// R2: after grants shrink — auto-promote queue head, or FloorIdle when
    /// everything is empty (promote emits FloorGranted ONLY per modules/01).
    fn promote_or_idle(next: &mut FloorState, out: &mut Vec<SignalingMessage>) {
        if next.grants.len() < next.capacity() && !next.queue.is_empty() {
            let head = next.queue.remove(0);
            next.grants.push(head.peer);
            next.grant_priority.push(head.priority);
            next.generation += 1;
            out.push(SignalingMessage::FloorGranted {
                grants: next.grants.clone(),
                generation: next.generation,
            });
            // shifted positions rebroadcast
            for (i, _) in next.queue.iter().enumerate() {
                out.push(SignalingMessage::FloorQueued {
                    position: i as u32,
                    generation: next.generation,
                });
            }
            return;
        }
        if next.grants.is_empty() && next.queue.is_empty() {
            out.push(SignalingMessage::FloorIdle {
                generation: next.generation,
                reason: None,
            });
        }
    }

}
