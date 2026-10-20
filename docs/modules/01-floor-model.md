# Floor Model — Domain Design
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Floor state machine (talkservo-core)

> **wave-1 note (2026-10-19)**: audit log / room registry / command authorization / role tiers / intervention kit are ADJUDICATED (D23 + docs/plans/2026-10-19-alpha-wave1-trust-control.md) — this section pre-dates wave-1; implementation has NOT started.

### 1.1 State machine (v2, D12)

```text
                    +--------------------------------------------+
                    |                                            v
Idle --Request--> Requesting/Queued --grant--> Granted --release--> Releasing --> Idle
   ^                 |    \--deny(closed enum)--> Idle                ^
   +--media-grace timeout / producer closed (holder) --+             |
   +-- ModeChange: ->Open drop queue; ->Exclusive: grants=[first]    |
preempt (strictly higher): old Granted -> FloorTaken -> new Granted  |
Leave(holder) -> immediate release -> auto-promote head -------------+
```

Canonical state (immutable updates; every transition bumps `generation`; grant-set model per D12 — replaces the single `holder` slot that could not represent Open/Hybrid):

```rust
pub struct FloorState {
    pub mode: FloorMode,
    pub grants: Vec<PeerId>,        // Exclusive/Hybrid: len <= cap(mode) (1 / 2 dual-floor); Open: every unmuted peer
    pub muted: Vec<PeerId>,         // Open-mode explicit overrides (authoritative, MuteSet-gated)
    pub queue: Vec<Pending>,        // dedup per peer; ordered: FIFO within tier, priority-desc across tiers
    pub generation: u64,            // monotonic; boot seed = random high offset (E7 stale-socket guard)
}

pub fn apply(state: &FloorState, event: &FloorEvent) -> (FloorState, Vec<SignalingMessage>)
// FloorEvent: Request{peer,priority,preempt} | Release{peer} | Leave{peer} | MediaDown{peer}
//           | Timeout | ModeChange{mode,actor} | MuteSet{peer,on,actor}
```

Rules (normative):

1. `Exclusive`: `grants.len() <= 1`; concurrent requests → `Queued` (never silent deny; queue position emitted via `FloorQueued`).
2. `Open`: everyone unmuted is granted. `MuteSet` (authorized) is a **privilege revocation on a room member**: it drops the peer from `grants` AND from `queue` (a queued position reserves the same privilege), then promotes the queue head if capacity frees. Un-muting restores the **privilege only** — the peer must request again; no floor is handed back (the seat may already belong to a promoted head, and re-adding would break the cap or pre-empt without priority). Re-scoped by the S7.5 walkthrough 2026-10-20; the pre-S7.5 text claimed mute merely toggled a roster the audio plane ignored.
3. `Hybrid`: `grants.len() <= 2` (MCPTT dual-floor); `ModeChange` transitions defined per direction: →Open drops queue + `FloorIdle`; →Exclusive keeps first-granted, others re-enter queue in prior order.
4. Pre-emption requires **strictly** `request.priority > grants[0].user_priority`; equal or lower → `Denied{PreemptPriority}` (never silent).
5. `request.priority > user ceiling` → `Denied{ExceedsCeiling}`.
6. Holder disconnect **or** media death (`MediaDown` from E11) → release after `floor_media_grace_ms` (dispatch profile default 2000; WS-drop path grace=0 stays); queue head auto-promotes. No zombie holders.
7. `floor_max_hold_ms`: default **45 s in dispatch profile**, off in Open mode; forced release → `FloorIdle{reason=timeout}`.
8. Release/Leave by a non-granted, non-queued peer = idempotent no-op. Repeated Request by grante = idempotent. **Mute is not gated on activity** (S7.5 A2): muting a silent member is a roster change with no generation bump and no floor event — the core holds no roster, so the "is that peer actually in this room" check lives in the server (`room.rs`), which drops a mute naming an absent id.
9. PoC starvation policy: none beyond cap (documented ceiling — revisit at Alpha with aging).
10. **Command authorization (plan-7 S3, 2026-10-19)**: `ModeChange` and `MuteSet` are AUTHORIZED commands — the server checks the sender's token role BEFORE core `apply()` (the state machine stays pure; denials never enter it). Dispatch role passes everything; non-Dispatch senders are denied with `FloorDenied{reason=unauthorized}` + an `authz_denied` audit line. Dispatch always outranks: a Dispatch target can never be muted by a non-Dispatch sender (no mutual-fight path). Token mod-claims (S4) OR into the room-level grant. FloorRequest/Release need no grant.

**Holder definition (D12)**: `holder` ≡ `grants[0]` and exists only in Exclusive/Hybrid (cap ≥1); Open mode has no holder — all unmuted peers are grantees. E11/F-sequence wording applies to grantees generally, holder-phrased for dispatch readability.

Priority model: user priority (static ceiling by role) ≥ request priority (per-request); preempt effective only when strictly above current grantees. `DenyReason { Busy, ExceedsCeiling, NotMember, RateLimited, PreemptPriority, NoMedia, Unauthorized }` — closed enum; every denial visible on the wire, silent drops forbidden (rate limit included). `Unauthorized` carries TWO meanings and is deliberately not split (adding a variant would break pre-S7.5 clients parsing a denial they have never seen): Rule 10 refuses an unauthorized COMMAND, and Rule 2 refuses a FLOOR REQUEST by a muted member. A client tells them apart by what it sent (a muted peer never issues ModeChange/MuteSet) and by whether its own id sits in the `muted` roster it mirrors — which is exactly the denied-vs-muted distinction modules/08 renders.
