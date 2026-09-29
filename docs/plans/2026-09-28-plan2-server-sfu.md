# Plan 2: Signaling Server + SFU Host — Task-Level Plan

> Skeleton plan: task decomposition, interfaces, and test names are pinned here; step-level expansion happens at execution start via the writing-plans skill. Prereq: plan-1 landed (apply()/wire signatures, pixi gates).

**Goal:** `talkservo-server` runs the full signaling surface (modules/02 sequences J/P/X/R/W/F) over a live `talkservo-sfu` host (mediasoup behind `sfu-mediasoup`, `stub-media` elsewhere), with E-matrix behaviors under integration test.

**Architecture:** room mailbox (single writer per process) routes wire messages → `core::apply` → broadcast + `sfu.apply_floor(state)` pause/resume diff (D12/D13). One supervised worker (round-robin seam ready, OQ-4).

**Revision:** v1.1 (2026-09-28) — +D16 routed findings section (queue purge, peer deltas, wire v/caps, room TTL)

**Spec:** docs/modules/02 (sequences), 03 (components/features), 05 (E1-E11), 06 (security/config), research/internal consolidated + arch-review2-consolidated.

**Global constraints:** all plan-1 constraints + `TS_JWT_SECRET` fail-fast, `/healthz`, 30s heartbeat, floor cooldown 500ms, E11 no-RTP watchdog 2s, config keys per modules/06 §config.

### Task 1: sfu host crate
Backend trait `ClientMediaBackend` (D15 client naming mirrors here): server side = `Sfu` trait (modules/03) over `mediasoup` (supervisor, per-room Router, transports, pause/resume diff `apply_floor`); `stub-media` no-op impl. Tests: supervisor spawn/kill cycle (Linux), stub builds on mac.
### Task 2: server core loop
Room mailbox + WS handler: Join (JWT validate → `Error{AlreadyJoined}` policy → Welcome{turnCreds}+RouterCaps), FloorRequest/Release/ModeChange/MuteSet → apply → broadcast incl. FloorQueued, role-scoped ServerSnapshot (R). Integration tests (stub): 2 fake WS clients drive J/P/X; gen discard; identity collision rejects.
### Task 3: media orchestration
TransportCreate/Connect/Produce/Consume handling per J-steps 3-7; Producer pause unless granted (D13); MediaRestart F/W. Tests: stub simulates lifecycle; mediasoup live path covered by Task 5/Linux.
### Task 4: timers + E-matrix
Heartbeat 30s → E1 release; E11 no-RTP watchdog + `floor_media_grace_ms`; max-hold (45s dispatch config); cooldown→`Denied{RateLimited}`; gen boot offset. Tests: fake-clock driven, one per E row.
### Task 5: live integration + deploy files
Linux-only: real worker test (create room, 2 transports, produce silence, grant→audio RTP flows, E6 kill worker, E11 kill uplink); `docker/{Dockerfile,compose.yml,coturn.yml}` with pinned coturn flags; compose healthcheck on /healthz.
### Task 6: gates + ledger (+ canonical WS fixtures: record J/P/X/R/W/F message transcripts as JSON fixtures consumed by plan-3 T1 vitest — cross F3)
`pixi run lint/test/test-sfu/audit` green; plan-3 interfaces recorded (`ServerApi` surface list); status/modules sync.

**Review Focus:** WS frame size limits (max message bytes, reject) · partial JSON frames · two rapid Joins same token · produce before TransportConnect ordering violation · token from "future" (nbf) skew.

## Round-2 routed findings (land during expansion, D16)

- Queue purge: peer_left removes queue entry + position rebroadcast (CM-2/#1); F-seq covers non-holders
- PeerJoined{info,gen} / PeerLeft{peer,gen} deltas; PeerList full-replace carries gen (CM-3/S-2, #4)
- Wire `v` field on Join, echoed in Welcome; undeserializable → `Error{BadVersion}` + close (CM-5/#9)
- Caps: `MAX_PEERS=10 MAX_QUEUE=8 MAX_FRAME_BYTES=64KiB`; overflow → Denied/413 (CM-6/#10)
- `Sfu::peer_left` cascade contract: transports+producers+consumers closed, queue purged, MediaDown emitted (CM-4/#11)
- JWT claims shape `{sub,room,role}` + field-role MuteSet/ModeChange rejection test (S-6/#14)
- ROOM_IDLE_TTL_S=600 reap → Router destroy (CM-1/#8, D16)

## plan-review additions (2026-09-28, land during expansion)

1. Task 6 gains **observability bootstrap**: tracing JSON + closed event vocab + room/peer/gen fields; log-derived `grant_latency_ms`/`media_recovery_ms` (feeds acceptance #7/#10)
2. NO `ClientMediaBackend` trait on server (D15 is client-crate): server = feature-gated `Sfu` impls only
3. E11 seam named: `Sfu` gains media-activity input (mediasoup AudioLevelObserver; stub emits synthetic activity)
4. Test-feature combo pinned: stub suite = `--no-default-features --features talkservo-server/stub-media`; live suite behind `pixi run test-sfu` (Linux)
5. TokenRefresh proactive push = Task 4 timer (expiry-5min threshold)
6. E4 → Task 3 (transport-create path), E10 → Task 3 (capacity guard)
7. `scripts/issue-token.sh` + `pixi run issue-token` = Task 5 deliverable
