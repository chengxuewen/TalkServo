# Plan 2: Signaling Server + SFU Host — Step-Level Plan

> **Revision:** v2.0 (2026-09-29) — expanded from skeleton to step level at execution
> start; absorbs all routed findings: round-2 ×7 (D16), plan-review ×7, Review-Focus
> pins ×6. Prereq plan-1 LANDED (apply()/wire/pixi gates all green).
>
> **Goal:** `talkservo-server` runs the full signaling surface (modules/02 sequences
> J/P/X/R/W/F) over a live `talkservo-sfu` host (mediasoup behind `sfu-mediasoup`,
> `stub-media` elsewhere), with E-matrix behaviors under integration test.
>
> **Architecture:** room mailbox (single writer per room) routes wire messages →
> `core::apply` → broadcast + `sfu.apply_floor(state)` pause/resume diff (D12/D13).
> One supervised mediasoup worker (round-robin seam noted, OQ-4 deferred).
>
> **Spec:** modules/02 (sequences, wire v:1) · 03 (Sfu trait, feature block) ·
> 05 (E1-E11, observability event vocab) · 06 (config keys, auth ladder) ·
> D16 routed findings.

## Global Constraints (inherit plan-1 + plan-2 additions)

- All plan-1 constraints hold (C1 English, D5 dep limits, serde tag rules, clippy/deny/coverage gates, C1 check before every commit).
- `TS_JWT_SECRET` env — **fail-fast at startup** (no default, panic with actionable message).
- Config keys (modules/06, env-driven, defaults): `TS_JWT_TTL_S=3600 TURN_TTL_S=3600 FLOOR_MAX_HOLD_MS=45000|off(dispatch)|open FLOOR_MEDIA_GRACE_MS=2000 FLOOR_REQUEST_COOLDOWN_MS=500 HEARTBEAT_S=30 RTC_PORT_MIN=40000 RTC_PORT_MAX=40100 TRANSPORT_GUARDRAIL=50 NO_RTP_WATCHDOG_MS=2000 ROOM_IDLE_TTL_S=600 MAX_PEERS=10 MAX_QUEUE=8 MAX_FRAME_BYTES=65536`.
- WS frame limit `MAX_FRAME_BYTES=64KiB` — reject + close 1009. Partial JSON frames = single WS message boundary (no streaming parse).
- JWT claims `{sub, room, role: dispatch|field}` HS256; `exp` + `aud=room` enforced.
- Wire `v:1`; undeserializable version → `Error{code:"bad_version"}` + close 4400.
- Test-feature combo (P2-#4): stub suite = `cargo test --workspace --no-default-features --features talkservo-server/stub-media`; live mediasoup suite = `pixi run test-sfu` (Linux only).

---

### Task 1: sfu host crate — Sfu trait + supervisor + apply_floor

**Files:** `crates/talkservo-sfu/src/{lib,host,stub}.rs` (+ `mediasoup_host.rs` behind feature)

**Interfaces (pin now, consumed by Task 2/3):**
- `Sfu` trait (modules/03 §trait, expanded):
  - `async fn create_transport(&self, room: &RoomId, peer: &PeerId) -> Result<TransportInfo, SfuError>`
  - `async fn connect_transport(&self, room, peer, dtls: Value) -> Result<(), SfuError>`
  - `async fn produce(&self, room, peer, rtp_params: Value) -> Result<ProducerId, SfuError>`
  - `async fn consume(&self, room, peer, producer_id) -> Result<(ProducerId, Value), SfuError>` (stub returns canned)
  - `async fn apply_floor(&self, room: &RoomId, state: &FloorState)` — **idempotent diff**: resume granted holders' producers, pause others (D13); consumers created/resumed to match grants; stub records call log for assertions
  - `async fn peer_left(&self, room, peer)` — **cascade**: close transports+producers+consumers (CM-4/#11)
  - `async fn media_activity(&self, room, peer) -> ActivityState` — **E11 seam (P2-#3)**: mediasoup = AudioLevelObserver; stub = synthetic (configurable script)
  - `async fn kill_worker(&self)` — E6 test hook (mediasoup impl only; stub no-op)
- `Supervisor` (mediasoup only): spawn worker → on `worker.exited` → spawn replacement + rebuild one Router per active room (W-sequence, modules/02 §W steps 1-2) → notify server via oneshot `WorkerRestarted{room}` channel
- `SfuError` (thiserror): `TransportTimeout`, `Capacity(String)`, `WorkerGone`, `Dtls(String)`

**Steps:**
- [ ] **1.1** trait + error + stub impl (records call log; `apply_floor` asserts diff order; `media_activity` scriptable) — stub compiles on any platform
- [ ] **1.2** failing test (stub): `apply_floor` pause/resume diff call log; `peer_left` cascade closes all
- [ ] **1.3** mediasoup host: Supervisor + Router registry + transport/produce/consume wiring per J-steps 3-7; `apply_floor` diff over producers (D13 gating)
- [ ] **1.4** `pixi run check` green (default = mediasoup, Linux); `--no-default-features --features talkservo-server/stub-media` green (mac path)
- [ ] **1.5** live smoke (Linux, Task 5 verifies full): supervisor spawn/kill cycle green
- [ ] **1.6** commit `feat(sfu): Sfu host trait + supervisor + stub/mediasoup backends`

### Task 2: server core loop — room mailbox + WS + auth + observability bootstrap

**Files:** `crates/talkservo-server/src/{main,config,auth,room,ws,obs}.rs`

**Steps:**
- [ ] **2.1** `config.rs`: env-driven `Config::from_env()` with all modules/06 keys + fail-fast `TS_JWT_SECRET` (missing → panic with fix hint). Unit test: defaults + missing-secret panic (`#[should_panic]`)
- [ ] **2.2** `auth.rs`: HS256 validate `{sub,room,role}` + `aud=room` + `exp`; role enum ↔ wire Role. Test: valid/expired/wrong-aud/wrong-role-shape (S-6 shape test)
- [ ] **2.3** `obs.rs` (P2-#1): `tracing_subscriber` JSON to stdout, level policy per modules/05 §Obs; **closed event vocabulary** as `Event` enum (`join leave floor_request grant deny taken release idle_timeout mode_change transport_create transport_fail produce_ok consume_ok worker_exited media_restart_done rate_limited`); every line carries `room`/`peer`/`gen` fields
- [ ] **2.4** `room.rs` mailbox: `tokio::sync::mpsc` + single writer task per room; state = `FloorState` + roster `HashMap<PeerId, PeerInfo{role, transport state}>` + `generation` from core; broadcast fan-out per role (dispatch sees queue messages, field doesn't)
- [ ] **2.5** `ws.rs` handler: axum `WebSocketUpgrade`; per-connection read loop with `MAX_FRAME_BYTES` reject (close 1009); Join sequence per modules/02 §J: JWT → `Error{AlreadyJoined}` if identity live → `Welcome{v:1, peer_id, turn_creds}` + `RouterCaps` + `PeerList{gen}` + role-scoped `ServerSnapshot`; second-Join-same-token rapid probe (Review Focus) = AlreadyJoined
- [ ] **2.6** floor routing: FloorRequest/Release/ModeChange/MuteSet → cooldown check (P2-#5 cooldown 500ms → `Denied{RateLimited}`) → `core::apply` → broadcast → `sfu.apply_floor(state)` → log `grant`/`deny`/`taken` events with `grant_latency_ms`
- [ ] **2.7** Resync (R1): send role-scoped `ServerSnapshot` + fresh `PeerList`; stale-gen discard is client-side (E2), server just serves truth
- [ ] **2.8** Room-reap idle task (D16): `ROOM_IDLE_TTL_S=600` no-members → sfu router destroy + log
- [ ] **2.9** integration tests (stub-media): 2 fake WS clients drive J→P→X; **gen discard** (E2: inject stale-gen message server-side); identity collision; `bad_version` close; frame-limit close; dispatch-vs-field snapshot scoping (queue visibility)
- [ ] **2.10** commit `feat(server): room mailbox + WS signaling + JWT + obs bootstrap`

### Task 3: media orchestration — J-steps 3-7 + E4/E10 + W/F triggers

**Files:** `crates/talkservo-server/src/media.rs`, extend `ws.rs`/`room.rs`

**Steps:**
- [ ] **3.1** TransportCreate → `sfu.create_transport` → `TransportInfo` (E4: timeout → retry 1 → `MediaFailed{peer}` kick media half; **E10**: `TRANSPORT_GUARDRAIL=50` exceeded → `Error{code:"media_capacity"}` capacity guard) — review #6 homes
- [ ] **3.2** TransportConnect (dtls) → produce-before-connect ordering violation → `Error{code:"bad_order"}` (Review Focus)
- [ ] **3.3** Produce → `sfu.produce` → `ProduceOk` → producer **paused unless granted** (D13, via apply_floor diff); Consume → `ConsumeOk{producer_id, rtp_parameters}`
- [ ] **3.4** W-sequence triggers: on `WorkerRestarted` channel → rebuild routers per room → broadcast `MediaRestart{room, reason}` → on each re-`ProduceOk` re-run `apply_floor(currentState)` (modules/02 §W steps 2-4); `media_recovery_ms` log on completion
- [ ] **3.5** F-trigger: `MediaDown{peer}` from E11 watchdog (Task 4) → `core::apply` → `MediaFailed{peer}` broadcast + promote
- [ ] **3.6** integration tests (stub): produce-before-connect reject; E10 capacity; E4 simulated timeout→MediaFailed; W simulated worker-restart → MediaRestart broadcast + apply_floor re-run count
- [ ] **3.7** commit `feat(server): media orchestration — J-steps 3-7, E4/E10, W/F triggers`

### Task 4: timers + E-matrix behaviors

**Files:** `crates/talkservo-server/src/timers.rs`

**Steps:**
- [ ] **4.1** heartbeat 30s → E1 (holder WS drop → release+promote via `Leave` mapping); dead-peer sweep emits `PeerLeft{peer,gen}` delta (D16)
- [ ] **4.2** E11 watchdog: NO-RTP 2s (`NO_RTP_WATCHDOG_MS`) via `sfu.media_activity` seam → grace `FLOOR_MEDIA_GRACE_MS=2000` → `MediaDown` → release+promote (modules/02 §F); stub `media_activity` scriptable for tests
- [ ] **4.3** max-hold timer: `FLOOR_MAX_HOLD_MS=45000` (dispatch) / off (open) → `Timeout` event
- [ ] **4.4** cooldown 500ms between FloorRequests per peer → `Denied{RateLimited}` (P2-#5; event `rate_limited`)
- [ ] **4.5** TokenRefresh proactive push (P2-#5): timer at `exp-5min` → `TokenRefresh{jwt}`; server re-signs with same claims
- [ ] **4.6** tests (fake-clock `tokio::time::pause`): one per E row — E1 heartbeat drop, E11 watchdog+grace, cooldown deny, max-hold fire, TokenRefresh at threshold, E2 stale-gen discard
- [ ] **4.7** commit `feat(server): timers + E-matrix — heartbeat, E11 watchdog, cooldown, max-hold, TokenRefresh`

### Task 5: live integration + deploy files + token issuance

**Files:** `tests/live_mediasoup.rs` (server crate), `docker/{Dockerfile,compose.yml,coturn.yml}`, `scripts/issue-token.sh`, `config/sample.env`

**Steps:**
- [ ] **5.1** `scripts/issue-token.sh` (P2-#7): reads `TS_JWT_SECRET`, mints HS256 `{sub,room,role,exp}` via python (no new deps); `pixi run issue-token` task wiring
- [ ] **5.2** live test (Linux, `sfu-mediasoup`): real worker — create room, 2 peers × transports, produce silence (opus frame), grant → assert `apply_floor` resumed holder producer; `kill_worker` (E6) → W-sequence → `MediaRestart` broadcast within budget; E11 synthetic silent-uplink
- [ ] **5.3** `docker/Dockerfile` (ubuntu:22.04 + pixi build chain), `compose.yml` (server + coturn, healthcheck `/healthz`, UDP 40000-40100 mapping, TS_JWT_SECRET from env file), `coturn.yml` (pinned flags per modules/06; plan-review LOW note: this is the coturn.yml home — **supersedes the "P0-1" mention in modules/04, annotate there**)
- [ ] **5.4** `config/sample.env` — all modules/06 keys with defaults, no secrets
- [ ] **5.5** `pixi run test-sfu` green (Linux); commit `feat(server): live integration + deploy files + issue-token`

### Task 6: gates + ledger + plan-3 handoff

**Steps:**
- [ ] **6.1** full sweep: `pixi run lint && pixi run test && pixi run audit && pixi run coverage` + `pixi run test-sfu` (Linux)
- [ ] **6.2** C1/C2/link gates
- [ ] **6.3** `ServerApi` surface list recorded for plan-3 (cross F3's fixtures deliverable = Task 2.9/3.6 transcripts) — write to `docs/reference/server-api.md` (living reference, C2)
- [ ] **6.4** modules/04 coturn annotation (supersede note per P2-LOW); status.md sync (crates table test counts, Phase 1+2)
- [ ] **6.5** commit `chore: plan-2 exit — gates green, ServerApi recorded`

## Review-Focus pins (from plan header, executed above)

| Pin | Where |
|-----|-------|
| frame-size limit + reject | 2.5 |
| partial JSON / single-frame parse | 2.5 (WS message boundary) |
| two rapid Joins same token | 2.5 (AlreadyJoined) |
| produce-before-connect ordering | 3.2 |
| token nbf skew | 2.2 (validate `nbf` too) |
| cooldown → RateLimited | 2.6/4.4 |
| E4→T3 / E10→T3 homes | 3.1 |
| E11 seam `media_activity` | 1.1/4.2 |
| stub-vs-live feature combo | Global Constraints |
| issue-token.sh | 5.1 |
| obs event vocab + metrics | 2.3/2.6/3.4 |
| ClientMediaBackend NOT invented | 1.1 (Sfu impls only — P2-#2) |

## Deliberately OUT (plan-3): grace-timer client UX, AlreadyJoined displacement mode, admin kill-switch endpoint (needs admin surface decision — plan-3 T5 dependency note), rolling upgrade, metrics server
