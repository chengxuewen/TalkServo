# Architecture Review — Consolidated Findings (Lead Merge)

> 2026-09-28 | arch-review-team: 4 dossiers merged by lead (raw 46 → 21 unique). Method: floor(F)/media(M)/ops(O)/client(C) lanes cross-deduped; severities re-anchored to "would implementation per current docs ship a failure or block acceptance". Evidence per finding = source dossier file:line; nothing here without a dossier citation.

## CRITICAL (acceptance/behavior guaranteed broken)

| # | Finding | Sources | Fix (one-liner) |
|---|---------|---------|-----------------|
| 1 | **Zombie holder**: holder's media dies while WS lives → floor stays Granted forever (W-seq rebuilds only on ProduceOk; E4/E5 floor-impact "none"; max-hold off) → silent room, queue blocked behind dead air. Worst failure for dispatch | F-01 ≡ M-3 (2/4 lanes) | transport-closed/`MediaFailed{peer}` → auto-release after `floor_media_grace_ms` (on-by-default); hooks exist in crate (iceConsentTimeout 30s, icestatechange); + acceptance item |
| 2 | **No TLS/wss story anywhere** → browsers gate getUserMedia on secure context → public acceptance #9 physically impossible over plain ws://http | O-F4 | modules/06 transport-security line (Caddy or axum-rustls; self-signed OK for PoC); compose + docs |

## HIGH

| # | Finding | Sources | Fix |
|---|---------|---------|-----|
| 3 | FloorState cannot represent Hybrid/Open multi-speaker/mute (holder: Option<PeerId>); Rule 2/3 claims unimplementable; MCPTT dual-floor precedent | F-02 | model grant-set (`speakers: Set`) or demote Hybrid to policy-switch in modules/01 — decision needed |
| 4 | ModeChange/mute-all/preempt-broadcast = UI promises with **no wire message, no FloorEvent** | F-03 | add client→server ModeChange{mode}/MuteSet{peer,on} + events + authority gate |
| 5 | No FloorQueued event → flagship queue strip has no live data; standards dossier already ruled "adopt now" | F-04 | `FloorQueued{pos,gen}` + queue-delta broadcast |
| 6 | apply_floor specified **three ways** across modules/02·03·04 (create/close vs pause/resume diff) | M-4 | unify: create-once-per-pair, apply_floor diffs pause state (crate API verified); ≤300ms budget depends on this |
| 7 | M1 publish-always + DTX-off + 2ch = always-hot decryptable uplink per idle peer; frozen rationale ("floor gates transmission") false | M-2 (+M-1 trust facet) | Producer::pause/track-disable off-floor, keep transport; mono (M-10); DTX revisit; O-F1 next turn if not |

## MEDIUM

| # | Finding | Sources | Fix |
|---|---------|---------|-----|
| 8 | Server operator holds plaintext of ALL peers (mediasoup terminates DTLS-SRTP; no E2EE) — trust assumption unstated | M-1 | trust statement modules/06 + OQ-10 (E2EE/SFrame post-PoC) |
| 9 | ServerSnapshot unscoped: field-agent reconnect leaks queue metadata (who is pending) | C-8 | per-role snapshot payload defined in modules/02 pre-code |
| 10 | Multi-tab same-identity join: no server policy (double producer/roster ambiguity) | C-9 | reject `Error{already-joined}` PoC-side (one core check); displace=Alpha decision |
| 11 | Token expiry: no refresh channel (wire silent, no REST); E9 UX undefined | C-6 | server→client `TokenRefresh{jwt}` (additive) + silent-refresh UX |
| 12 | Queue ordering/starvation/ModeChange-queue semantics undefined | F-05, F-06 | FIFO-within-tier, priority-across; one line per switch direction |
| 13 | DenyReason open-ended; rate-limited request outcome invisible | F-07 | closed enum incl. RateLimited, PreemptPriority — never silent-drop |
| 14 | max-hold default-off in a dispatch product (stuck PTT = floor monopolized) | F-08 | dispatch profile default 30-60s |
| 15 | TCP fallback: same-range TCP ports must be open; priority order wrong (real 4G path = TURN-TLS/443); ICE-Lite = no restart | M-6 | candidate order UDP→relay-TLS→ICE-TCP; compose open TCP range; covered by #1 |
| 16 | E10 "~50 transports" invented capacity claim; worker placement hardcoded (crate has worker_manager round-robin) | M-5 | "config guardrail, measure at acceptance"; placement behind supervisor iface now |
| 17 | /healthz missing (compose healthcheck + CI readiness need it) | O-F8 | one axum route |
| 18 | coturn config not pinned (flags/realm/nonce 600s vs 1h TTL) | O-F5 | docker/coturn.yml at P0-1 pins them |
| 19 | Electron text misses single-instance lock, render-process-gone recovery, Linux auto-update form; C-4 register() conflict UX | C-7, C-4 | 3 lines in D10/modules/08 |
| 20 | join/connect flood has no limit (only floor-request cooldown) | O-F6 | accept+document PoC risk OR one middleware |
| 21 | pixi.lock commit rule unstated (toolchain float = mediasoup build-pit surface) | O-§3 | one line modules/09 |

## LOW / recorded (no action now)
O-F1 key rotation (Alpha) · O-F7 SIGTERM drain (Alpha) · F-09 gen on Taken/Denied · F-10 diagram queue paths · F-11 preempt-insufficient outcome · F-12 E7 gen-reset · F-13 room cardinality invariant · C-1 browser matrix text · C-2 sinkId matrix (my dispatch premise corrected by BCD) · C-3/C-5/C-11 impl notes · C-10 i18n product question · M-7 multi-homed listenInfos · M-8 BWE · M-11 crate 0.24→0.28.1 drift plan · **M-9 RFC 8215 mis-cite in frozen dossier → PIT-4 recorded**.

## What held up (multi-lane confirmed)
R1 snapshot-resync · W-seq vs mediasoup died→recreate · generation-as-race-guard · netem quantitative A/B (stronger than any surveyed precedent) · TURN short-TTL creds · Router-as-dumb-forwarder seam · mediasoup crate = official in-tree (bus-factor ≈3, Versatica) — supply-chain verdict HEALTHY.

## Sources
- Dossiers: architecture-review-{floor,media,ops,client}.md (this dir) — every row cites them; external evidence lives there.
