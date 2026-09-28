# Floor Model — Domain Design
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Floor state machine (talkservo-core)

### 1.1 State machine

```text
                 +----------------------------+
                 |                            v
Idle --Request--> Requesting --grant--> Granted --release--> Releasing --> Idle
                 |             \--deny--> Denied --> Idle       ^
                 +--preempt(strictly higher)--> [old holder: FloorTaken]
```

Canonical state (immutable updates; every transition bumps `generation`):

```rust
pub struct FloorState {
    pub mode: FloorMode,
    pub holder: Option<PeerId>,
    pub queue: Vec<Pending>,      // dedup per peer; auto-promote head on release/leave
    pub generation: u64,          // monotonic; clients discard <= seen
}

pub fn apply(state: &FloorState, event: &FloorEvent) -> (FloorState, Vec<SignalingMessage>)
// FloorEvent: Request{peer,priority,preempt} | Release{peer} | Leave{peer} | Timeout
```

Rules (normative):

1. `Exclusive`: at most one Granted holder; concurrent requests queue or deny per policy.
2. `Open`: implicit grants; explicit mute/permission overrides tracked in state.
3. `Hybrid`: `ModeChange` (authorized) switches mode; in-flight grants survive the switch.
4. Pre-emption requires **strictly** `request.priority > holder.user_priority`; equal never pre-empts (MCPTT-aligned).
5. `request.priority > user ceiling` → `Denied{ExceedsCeiling}`.
6. Holder disconnect → immediate release (PoC grace = 0); queue head auto-promotes.
7. `floor_max_hold_ms` (default off): forced release → `FloorIdle{reason=timeout}`.
8. Release/Leave with no holder = idempotent no-op. Repeated Request by holder = idempotent.

Priority model: user priority (static ceiling by role) ≥ request priority (per-request); preempt flag only effective when strictly above holder's.
