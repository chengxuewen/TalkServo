# Architecture Review Dossier: Floor Control (modules/01+02) — Adversarial Findings

> **Status: FROZEN review dossier (C2).** Point-in-time review of the live design; findings are proposals for the design owners — this file never back-writes modules/. Date: 2026-09-28 | Reviewer: floor-reviewer (arch-review-team). Rule C1: English only.
> Method: local design docs read in full; external precedent = fetched today where network allowed (mediasoup API docs, talktome README via api.github.com) + internally-verified frozen dossiers (standards-ptt-mcptt.md = 3GPP text fetched 2026-09-28; oss-voice-infrastructure.md = repo/docs verified 2026-09-28). mumble.info protocol page ETIMEDOUT, Janus doxygen JS-rendered, LiveKit client docs 404 → those specifics marked UNVERIFIED.

## Findings

**F-01 | CRITICAL | defect — zombie holder: floor survives its own media plane forever.**
Claim: W-seq rebuilds consumers only "on each ProduceOk" (modules/02:23); if the Granted holder never re-produces (mic revoked, E4 retry exhausted, hung tab with live WS), FloorState stays Granted, apply_floor forwards nothing, the room is silent and everyone queues behind a dead-air holder. E4/E5 floor impact = "none" (modules/05:13-14) and floor_max_hold_ms default off (modules/01:38) → no release path exists.
Precedent: mediasoup delegates all floor logic to app signaling (oss-voice-infrastructure.md:17) — producer-death→floor-release is explicitly the app's job, and our app doesn't do it; LiveKit server-side mutes dead/unpublished tracks (same:16).
Recommendation: server hook — producer closed/absent while holder → release floor after a bounded `floor_media_grace_ms` (default on for dispatch); add acceptance item ("kill holder mic, keep WS → floor frees within grace").

**F-02 | HIGH | defect — FloorState cannot represent Hybrid/Open multi-speaker or mute state.**
Claim: `holder: Option<PeerId>` is one slot (modules/01:21), yet Rule 2 says Open-mode "mute/permission overrides tracked in state" (no such field) and Rule 3/Hybrid (modules/01:34; architecture.md:19) implies simultaneous speakers. MCPTT's dual-floor — the standardized hybrid moment our dossier maps to (standards-ptt-mcptt.md:37,65) — has two concurrent talkers; our data model can't hold it, so `apply()` and `ServerSnapshot{state}` (modules/02:9) are unrepresentable for Hybrid.
Precedent: Mumble per-channel Talk ACL + LiveKit canPublish grants model "set of permitted speakers", not one holder (oss:15-16).
Recommendation: either extend state (`speakers: Set<PeerId>` / `muted: Set<PeerId>`, grant-set semantics) or explicitly demote Hybrid to "Exclusive/Open policy switch only" in modules/01 until modeled.

**F-03 | HIGH | defect — ModeChange / mute-all / preempt-broadcast have no wire message and no FloorEvent.**
Claim: modules/06:19 gates "ModeChange/mute overrides" behind elevated priority and modules/08:10 ships them as dispatcher buttons, but client→server wire (modules/02:8) contains none of them and `FloorEvent` = Request|Release|Leave|Timeout (modules/01:27) — `apply()` cannot produce these transitions; the actions are UI promises with no protocol path.
Precedent: Janus AudioBridge exposes admin `mute`/`mute_room` as first-class protocol commands (oss:18); talktome "Talk Lock" is a server-side talk-permission command (README via api.github.com, fetched 2026-09-28).
Recommendation: add `ModeChange{mode}` / `MuteSet{peer,on}` client→server messages + matching FloorEvent variants; enumerate who may send (ties to F-07 reasons).

**F-04 | HIGH | defect — no FloorQueued event: queued peers get zero feedback and the flagship queue strip has no live data.**
Claim: server→client set (modules/02:9) has no queued/position message; queue is only visible inside `ServerSnapshot` at Join time. The dispatcher's "grant-queue strip" — our declared industry differentiator (modules/08:10; ptt-ui-patterns.md:95,113) — would show stale data between joins. The frozen standards dossier already ruled this "adopt now" (standards-ptt-mcptt.md:62,74: explicit queued event + position).
Precedent: MCPTT Floor Queue carries queue-position info, verified in TS 24.380 text (standards:39); Nextcloud raise-hand notifies moderators (oss:21).
Recommendation: add `FloorQueued{pos,gen}` to requester + queue-delta broadcast (or embed queue summary in FloorGranted/FloorIdle).

**F-05 | HIGH | gap — queue ordering and starvation are unspecified.**
Claim: `queue: Vec<Pending>` with "auto-promote head" (modules/01:22) never defines the order (insertion? priority?) and has no aging — a stream of higher-priority requests starves a queued peer indefinitely, invisibly (see F-04). Rule 1's "queue or deny per policy" (modules/01:32) also never names the policy.
Precedent: MCPTT's signaled queue position implies a defined server-maintained order (standards:39); no fetched source for aging in any surveyed system — UNVERIFIED as convention, hence flagged as "define", not "adopt X".
Recommendation: normative rule: FIFO within equal priority, priority-desc across tiers; state the starvation ceiling explicitly (even if "none in PoC").

**F-06 | MEDIUM | gap — ModeChange queue semantics undefined.**
Claim: Rule 3 keeps in-flight grants across a mode switch (modules/01:34) but says nothing about `queue`: Exclusive→Open makes queued requests moot (grant-all? drop?); Open→Exclusive with N implicit speakers — who keeps the single holder slot? Hybrid→Exclusive with dual speakers is currently unrepresentable (F-01/F-02).
Precedent: MCPTT re-negotiates floor state on call-type change/regroup rather than silently carrying it (standards:38,81 — regroup deferred there too, but the state question is answered by an explicit procedure).
Recommendation: one normative line per direction: →Open drop queue + broadcast FloorIdle; →Exclusive first-granted survives, rest enter queue in request order.

**F-07 | MEDIUM | gap — DenyReason enum unenumerated; rate-limited requests have no defined client-visible outcome.**
Claim: floor-request rate limit 500 ms exists (modules/06:20) and logs `rate_limited` (modules/05:29), but the wire has only `FloorDenied{reason}` (modules/02:9) with reasons scattered ad hoc (`ExceedsCeiling` modules/01:36, `NotMember` modules/05:17) and no closed enum (modules/03:27 names `DenyReason` without values). A silently-dropped rate-limited Request leaves the client keying into a void (no ack, at-most-once — modules/02:18).
Precedent: MCPTT Floor signalling carries a typed Reject Cause field (standards:39,69).
Recommendation: enumerate `DenyReason {Busy, ExceedsCeiling, NotMember, RateLimited, PreemptPriority, …}`; rate-limit → `Denied{RateLimited}`, never silent drop.

**F-08 | MEDIUM | improvement — max-hold default-off leaves dispatch hot-mic/stuck-transmitter undefended.**
Claim: Rule 7 `floor_max_hold_ms` default off (modules/01:38) in a product whose beachhead is dispatch: a stuck PTT (hardware latch, JS crash mid-hold with live WS) monopolizes the Exclusive floor indefinitely; E1 only covers WS drop (modules/05:10).
Precedent: MCPTT defines a Duration field bounding talk spurts (standards:69); TETRA-class radios use hangtime/max-transmit conventions (standards:44-45, secondary sources).
Recommendation: default ON in the dispatch profile (30–60 s) → forced `FloorIdle{reason=timeout}`; keep off only for Open mode.

**F-09 | LOW | defect — FloorTaken/FloorDenied carry no `gen`, violating the generation contract.**
Claim: "every transition bumps generation; clients discard <= seen" (modules/01:23) but `FloorTaken{by}` and `FloorDenied{reason}` have no gen field (modules/02:9). X-seq pairs Taken+Granted in-order (modules/02:17) so TCP saves it within a connection, but a post-resync replay edge or a lone Taken (preempt that later fails) leaves clients unable to discard correctly.
Our-design evidence: modules/02:9 — Granted/Idle have gen, Taken/Denied don't.
Recommendation: gen on every state-bearing broadcast now (additive-only evolution makes retrofitting a compat problem later).

**F-10 | LOW | gap — state-machine diagram omits the queue paths its own rules require.**
Claim: §1.1 diagram (modules/01:9-14) has no Queued state/transition although Rule 1 queues and FloorState.queue exists — the normative surface under-specifies apply(); MCPTT's client state machine includes an explicit `queued` state (standards:35).
Recommendation: extend diagram: Requesting→Queued (occupied), Queued→Granted (promote), Queued→Idle (withdraw/leave/ceiling).

**F-11 | LOW | gap — pre-empt with insufficient priority has undefined outcome.**
Claim: Rule 4 requires strictly-higher priority for preempt (modules/01:35) but never says what happens to `Request{preempt=true, priority<=holder}` — queue, deny, or drop? A dispatcher hammering barge-in gets undefined behavior per F-07.
Recommendation: normative: `Denied{PreemptPriority}` (ties into F-07 enum).

**F-12 | LOW | gap — server restart (E7) resets generation; stale half-open sockets can mis-discard.**
Claim: PoC has no persistence; E7 rebuilds rooms empty (modules/05:16,21) so gen restarts low while clients "discard <= seen" (modules/01:23) — surviving half-open WS sessions (up to 30 s heartbeat window, modules/05:10) discard post-restart broadcasts until their next Join/snapshot. Narrow, but free to close.
Recommendation: on boot, start gen from a random/high offset or push ServerSnapshot to any still-open socket after room rebuild; one line in modules/05.

**F-13 | LOW | gap (contract) — room identity rides entirely on JWT aud; one room per WS session is an unstated invariant.**
Claim: no client→server message names a room (modules/02:8); routes carry `:room` (modules/08:10-11) and JWT has a room-scoped audience (modules/06:15) — fine for PoC single-room (deliberate deferral, modules/08:24), but Alpha multi-channel dispatch (ptt-ui-patterns.md:106-109) needs N sockets or a room field, and "additive-only" evolution (modules/02:11) can't force a *required* field onto old clients.
Precedent: LiveKit one-room-per-token, Mumble one-session-full-channel-tree — both make the session↔room cardinality explicit (oss:15-16).
Recommendation: state the invariant ("one room per connection; multi-room = N connections") in modules/02, or add optional `room` with aud fallback now.

## What held up under attack
R1 snapshot-resync rationale (TCP in-order, connection-boundary loss covered) is sound and correctly distinguishes why MCPTT re-send timers don't apply (modules/02:18 ↔ standards:40). W-seq matches mediasoup's own died→recreate guidance ("worker.on('died')", mediasoup.org/documentation/v3/mediasoup/api/, fetched 2026-09-28) and the per-ProduceOk `apply_floor` diff is idempotent/self-healing (modules/03:35-40). Generation-as-race-protection has no standard equivalent but is correctly kept (standards:68). Priority ceiling model matches TS 24.380's static/request/effective split (standards:36).

## Sources
- Local (read in full 2026-09-28): docs/architecture.md; docs/modules/01-floor-model.md; docs/modules/02-signaling-protocol.md; docs/modules/03-components.md; docs/modules/05-error-model.md; docs/modules/06-deployment-security.md; docs/modules/08-web-ui.md; docs/reference/research/ptt/standards-ptt-mcptt.md (3GPP TS 24.379/24.380/23.280 facts, verified at fetch 2026-09-28); docs/reference/research/ui/ptt-ui-patterns.md §2(b); docs/reference/research/media/oss-voice-infrastructure.md:15-22 (Mumble/LiveKit/mediasoup/Janus/Jitsi arbitration rows).
- Fetched externally today: https://mediasoup.org/documentation/v3/mediasoup/api/ (worker 'died' event, Consumer pause/resume, PipeTransport); https://api.github.com/repos/thepoison606/talktome/readme (Talk Lock, feeds mute); https://api.github.com/repos/{versatica/mediasoup,meetecho/janus-gateway,mumble-voip/mumble,livekit/livekit}/readme.
- UNVERIFIED today (network-blocked, marked where cited): mumble.info protocol page (ETIMEDOUT); Janus doxygen plugin pages (JS-rendered 404); LiveKit client reconnection docs (404); TETRA/DMR hangtime specifics (secondary only, per standards dossier).
