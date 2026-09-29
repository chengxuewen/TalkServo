# P0-1: Workspace Toolchain + talkservo-core First Slice — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A committable Rust workspace (3 crates) with the full pixi toolchain gate, and `talkservo-core` carrying the complete floor domain (D12 state model + wire contract) under exhaustive unit test.

**Architecture:** Pure-domain crate (`talkservo-core`, no async/I-O per D5) compiled by a thin workspace; sfu/server crates exist as feature-gated skeletons whose gates run; all tooling via pixi per D11 (Linux-native-first, Docker = CI parity only).

**Tech Stack:** Rust stable (edition 2024, rust-toolchain.toml), pixi (conda-forge), serde/serde_json, thiserror, cargo-deny/clippy/tarpaulin.

**Revision:** v1.1 (2026-09-28) — +schema artifact step (review S-7)

**Spec:** docs/architecture.md v0.2 + docs/modules/01-floor-model.md + docs/modules/02-signaling-protocol.md + docs/modules/03-components.md + docs/modules/09-dev-toolchain.md (D1-D13).

## Global Constraints

- All artifacts English (C1); commit messages conventional (`feat:`/`chore:`/`test:`); `Cargo.lock` AND `pixi.lock` committed every change.
- Rust edition 2024; `talkservo-core` deps limited to `serde`, `serde_json`, `thiserror` — nothing else (D5).
- Feature block (single source of truth, modules/03): `default=["sfu-mediasoup"]`, `sfu-mediasoup=["dep:mediasoup"]`, `stub-media=[]`; compile-time assert: exactly one active.
- `generation` seeds at random u64 high-offset at state creation (review F-12); never wraps test assumptions into ordering-by-value.
- Every wire enum: `#[serde(tag="type", rename_all="snake_case")]`; field names snake_case; additive-only evolution (modules/02).
- Clippy `-D warnings`, `cargo deny check` clean, coverage ≥80% on `talkservo-core` (tarpaulin task).
- Opus codec facts (mono, 48k, FEC) are modules/04 pinned values — no channel/stereo literals in core (media config is sfu-crate scope).

## Review Focus

1. Duplicate `Request` by same peer while queued → idempotent re-queue (position unchanged) — test in Task 4.
2. Two tabs same identity → `Error{AlreadyJoined}` is server-domain; core only models it as `Join` rejection variant — keep in wire types Task 3, behavior test in plan-3 (server).
3. `generation` comparisons must never assume monotonic-across-restarts — Task 4 tests use fresh-state gen anywhere; boot-offset test in Task 4.
4. DenyReason closed enum: adding a variant later must not break clients — Task 3 test: unknown `reason` string deserializes into explicit `Unknown(String)`?? NO — closed enum: unknown reason → serde error is correct PoC behavior; test pins that debug-builds reject.
5. Feature mutual-exclusion compile assert — Task 2 step makes `--features stub-media` AND `--features sfu-mediasoup` fail to compile together on server/sfu crates.

---

### Task 1: Toolchain bootstrap files (pixi + entry scripts)

**Files:**
- Create: `pixi.toml` (per modules/09 §3 sketch verbatim: workspace name `talkservo`, platforms linux-64/osx-64/osx-arm64, deps rust>=1.85,<2 / python 3.12.* / nodejs>=22 / meson / ninja / clang / libclang / openssl / pkg-config / zlib / ripgrep / pyyaml; feature.dev.tasks: check/build/test/lint/format/format-fix/audit/coverage/test-sfu/web-install/web-build/run-server/run-coturn; feature.ci.tasks: ci-check/ci-check-mac/ci-lint/ci-test; activation env MESON/NINJA/LIBCLANG_PATH/PKG_CONFIG_PATH with the MESON_ARGS-unset comment citing MediaServo PIT)
- Create: `rust-toolchain.toml` (stable channel)
- Create: `deny.toml`, `clippy.toml`, `tarpaulin.toml` (minimal viable: deny = advisories+licenses ISC/BSD/MIT/Apache-2.0 allow; clippy = pedantic off, correctness on)
- Create: `scripts/_common.sh` (set -euo pipefail, log() helper, pixi-present guard, `pkill||true` discipline per edit-safety)
- Create: `bootstrap.sh` (two-stage entry per modules/09 §4: detect pixi → curl install → `pixi install` → print next-step banner; idempotent on crates-less checkout, exit 0)
- Create: `pixi.sh` (`exec pixi shell -e dev`)
- Create: `scripts/docker-cargo.sh` (CI-parity wrapper, ubuntu:22.04 image build if missing — mirrors sister pattern)
- Create: `scripts/scan-hardcode.sh` (grep for sk-/api_key/password=/port literals outside config/, excludes tests+node_modules; exits non-zero on hit — fulfills security.md's dangling reference)
- Modify: `.gitignore` (append `target/`, `dist/`, `node_modules/` (rooted), `.pixi/`, `out/`)

**Interfaces:**
- Produces: `pixi run <task>` names used by every later task: `lint`, `test`, `check`, `format`, `audit`, `coverage`.

- [ ] **Step 1: Write all files above** (this task is config, no test-first cycle)
- [ ] **Step 2: `bash -n bootstrap.sh pixi.sh scripts/*.sh`** — all pass
- [ ] **Step 3: `pixi install`** produces `pixi.lock`; commit BOTH (`pixi.toml`, `pixi.lock`)
- [ ] **Step 4: Commit** `chore: P0-1 toolchain — pixi workspace, bootstrap entry, gate configs`

### Task 2: Workspace + 3 crate skeletons + feature gate

**Files:**
- Create: `Cargo.toml` — `[workspace] members=["crates/talkservo-core","crates/talkservo-sfu","crates/talkservo-server"], resolver="3"`, `[workspace.package] edition="2024"`, `[workspace.dependencies] serde={version="1",features=["derive"]}`, serde_json, thiserror
- Create: `crates/talkservo-core/Cargo.toml` (deps: serde, thiserror only) + `src/lib.rs` with `#![deny(missing_docs)]`? No — plain lib + one smoke test
- Create: `crates/talkservo-sfu/Cargo.toml`: `[features] default=["sfu-mediasoup"] sfu-mediasoup=["dep:mediasoup"] stub-media=[]` + `src/lib.rs` with the compile-time mutual-exclusion assert:
  ```rust
  #[cfg(all(feature = "sfu-mediasoup", feature = "stub-media"))]
  compile_error!("exactly one media backend must be active");
  #[cfg(not(any(feature = "sfu-mediasoup", feature = "stub-media")))]
  compile_error!("no media backend active");
  ```
  plus a stub re-export module so `stub-media` builds standalone
- Create: `crates/talkservo-server/Cargo.toml` (deps core+sfu+axum+tokio-tungstenite+jsonwebtoken; same feature block pattern re-exporting sfu's) + `src/main.rs` printing version + `GET /healthz` placeholder axum route (returns 200 `{ok:true}`)
- Create: `.github/workflows/ci.yml` — jobs: `fmt` (cargo fmt --check), `check` matrix [ubuntu, macos] (mac: `--no-default-features --features stub-media`), `clippy` (-D warnings), `test` (ubuntu full), `test-mediasoup` (ubuntu, `cargo test -p talkservo-server --features sfu-mediasoup` — will pass with empty tests), `audit` (cargo-deny)
- Modify: `docs/README.md` — modules/09 planned-table row: scripts/landed note

**Interfaces:**
- Produces: workspace member names; `talkservo-server` binary answering `/healthz`; feature names `sfu-mediasoup`/`stub-media` (consumed by plan-3)

- [ ] **Step 1: Cargo.toml files + lib/main skeletons**
- [ ] **Step 2: `pixi run check` passes; `pixi run lint` zero warnings; `cargo check --workspace --no-default-features --features talkservo-server/stub-media` passes (mac path; P1-2: member-qualified form required on virtual workspace root — modules/09 form)**
- [ ] **Step 3: mutual-exclusion probe**: `cargo check -p talkservo-sfu --features sfu-mediasoup,stub-media` → compile_error fires
- [ ] **Step 4: `pixi run test` green; commit** `feat: workspace skeleton — core/sfu/server + feature gates + CI`

### Task 3: talkservo-core — identity + wire contract types

**Files:**
- Create: `crates/talkservo-core/src/lib.rs` (module decls), `src/ids.rs`, `src/wire.rs`, `src/error.rs`
- Modify: `crates/talkservo-core/Cargo.toml` (serde derive already from workspace deps)

**Interfaces:**
- Produces (exact, consumed by Task 4/5 and plan-3):
  - `pub struct PeerId(pub SmolStr)` — no, keep std: `pub struct PeerId(pub Arc<str>)`; `impl Display`; `PartialEq/Eq/Hash/Clone`
  - `pub struct RoomId(pub Arc<str>)`; same impls
  - `#[non_exhaustive] pub enum FloorMode { Exclusive, Open, Hybrid }`
  - `pub enum DenyReason { Busy, ExceedsCeiling, NotMember, RateLimited, PreemptPriority, NoMedia }` (closed, modules/01)
  - `#[serde(tag="type", rename_all="snake_case")] pub enum SignalingMessage { Join{jwt:String}, FloorRequest{priority:u8, preempt:bool}, FloorRelease, ModeChange{mode:FloorMode}, MuteSet{peer:PeerId, on:bool}, TransportCreate, TransportConnect{dtls:serde_json::Value}, Produce{rtp_parameters:serde_json::Value}, Consume{producer_id:serde_json::Value}, Resync, /* server→client: */ Welcome{peer_id:PeerId, turn_creds:serde_json::Value}, RouterCaps{media_codecs:serde_json::Value}, PeerList{peers:Vec<PeerInfo>}, FloorGranted{grants:Vec<PeerId>, generation:u64}, FloorTaken{by:PeerId, generation:u64}, FloorDenied{reason:DenyReason, generation:u64}, FloorQueued{position:u32, generation:u64}, FloorIdle{generation:u64, reason:Option<String>}, TransportInfo{ice:serde_json::Value, dtls:serde_json::Value, addrs:serde_json::Value}, ProduceOk{producer_id:serde_json::Value}, ConsumeOk{producer_id:serde_json::Value, rtp_parameters:serde_json::Value}, MediaRestart{room:RoomId, reason:String}, MediaFailed{peer:PeerId}, TokenRefresh{jwt:String}, ServerSnapshot, Error{code:String, detail:String} }`
  - `pub struct PeerInfo { id: PeerId, role: Role, connected_since_ms: u64 }`, `pub enum Role { Dispatch, Field }`
  - `pub enum ServerSnapshotPayload { Field(FieldSnapshot), Dispatch(DispatchSnapshot) }` — `FieldSnapshot{mode,grants,muted,generation,peers}` (NO queue), `DispatchSnapshot` adds `queue: Vec<Pending>` (D12 role-scoping; wire `ServerSnapshot` wraps payload + top-level `generation`)
- [ ] **Step 1: failing test** `tests/wire_roundtrip.rs`: for a representative sample (≥1 per variant incl. `FloorQueued{position:7,generation:99}`), `serde_json::to_value` → assert `["type"] == "floor_granted"`-style snake_case tags → `from_value` → equality
- [ ] **Step 2: run** `pixi run test -p talkservo-core` → FAIL (modules absent)
- [ ] **Step 3: implement types exactly as above**; `DenyReason` unknown-tag test: `{"type":"floor_denied","reason":"wat","generation":1}` deserialization errors (debug assert)
- [ ] **Step 4: run** → PASS; **Step 5: commit** `feat(core): identity + wire contract (modules/02 table, closed DenyReason)`

### Task 4: talkservo-core — FloorState + apply() (rules 1-9)

**Files:**
- Create: `crates/talkservo-core/src/floor.rs`, `tests/floor_rules.rs`

**Interfaces:**
- Consumes: ids + wire enums from Task 3
- Produces:
  - `pub struct FloorState { mode: FloorMode, grants: Vec<PeerId>, muted: Vec<PeerId>, queue: Vec<Pending>, generation: u64 }` (fields private; accessors) + `Pending { peer: PeerId, priority: u8 }`
  - `pub fn initial(mode: FloorMode) -> FloorState` — generation from `SystemTime` nanos (nonzero, high-entropy)
  - `pub fn initial_with_generation(mode: FloorMode, generation: u64) -> FloorState` — deterministic ctor for tests (P1-M: makes coverage gate deterministic; production path uses SystemTime)
  - casing pinned: wire fields snake_case (serde rename_all) — plan-3 TS unions mirror exactly; modules/02 camelCase shorthand is prose-only
  - `pub enum FloorEvent { Request{peer,priority,preempt}, Release{peer}, Leave{peer}, MediaDown{peer}, Timeout, ModeChange{mode}, MuteSet{peer,on} }`
  - `pub fn apply(state:&FloorState, ev:&FloorEvent) -> (FloorState, Vec<SignalingMessage>)` — emits wire messages per modules/02 (grant → FloorGranted+FloorQueued to demoted, etc.)
- [ ] **Step 1: failing tests** `tests/floor_rules.rs` covering rules 1-9 one test each, minimum set:
  - exclusive cap 1: second Request while granted → Queued with `FloorQueued{position:0}`; duplicate Request same peer → same position, no dup
  - queue order: FIFO within equal priority, priority-desc across tiers (higher-priority request jumps ahead; equal keeps arrival order)
  - release holder → auto-promote head → `FloorGranted` to promoted + `FloorIdle` to others? (NO — modules/01: promote emits `FloorGranted` only; Idle only on empty) — pin: promote→granted broadcast; empty→`FloorIdle`
  - `MediaDown{granted}` → removed after nothing (grace is server-timer domain; core: MediaDown releases immediately, grace enforced by server timer per modules/05 E11) — pin: core applies MediaDown as immediate release; the timer lives in server (plan-3)
  - `Timeout` → `FloorIdle{reason:Some("timeout")}`; mode switch Exclusive→Open drops queue + FloorIdle; Open→Exclusive keeps first-granted only, rest → queue in prior order; Hybrid cap 2 grants
  - preempt: strictly-higher → grants swap + `FloorTaken{by}` to old; equal/lower → `Denied{PreemptPriority}`; ceiling violation → `Denied{ExceedsCeiling}`
  - no-op cases: Release by non-granted, MuteSet non-member, Leave when idle — all idempotent, generation UNCHANGED on no-ops
  - queue purge (P1-4/D16): `Leave{peer}` when QUEUED → removed from queue, remaining positions rebroadcast via `FloorQueued{pos,gen}` to each affected peer; purged peer gets nothing (they left); disconnect path = server maps to Leave (core identical)
  - Open mode: MuteSet{peer,true} → peer out of grants, `muted` gains; MuteSet{false} → back in; unauthorized MuteSet is server-domain (core trusts actor)
  - boot gen offset: `initial()` twice → different generations, both nonzero; `initial_with_generation(m, 0)` → returns state with gen 0 (test-only)
  - caps enforcement (P1-M/D16): Request beyond `MAX_QUEUE` (passed as arg to apply via `FloorLimits{max_peers,max_queue}` — core stays config-free) → `Denied{Busy}`; peers beyond max handled server-side (core trusts PeerList)
- [ ] **Step 2: run** → FAIL
- [ ] **Step 3: implement apply()** — pure, no I/O; generation bumps on every real transition (no-op = same state, per modules/01 rule 8)
- [ ] **Step 4: run all + `pixi run coverage`** → PASS, core coverage ≥80%
- [ ] **Step 5: commit** `feat(core): FloorState v2 apply() — rules 1-9 under exhaustive test (D12)`

### Task 5: gates + ledger close-out

**Files:**
- Modify: `.agents/memorys/status.md` (Phase 1 core slice done; crates table row counts; next = plan-2 server/sfu)
- Modify: `docs/modules/09-dev-toolchain.md` (mark bootstrap/scripts as LANDED, not planned)
- Modify: `docs/README.md` planned-table (signaling-schema still planned)

**Interfaces:**
- Produces: verified gate commands as the P0-1 exit criteria

- [ ] **Step 1: full gate sweep**: `pixi run lint && pixi run test && pixi run audit && pixi run coverage` all green
- [ ] **Step 2: C1/C2/link gates** (AGENTS.md COMMANDS block) clean
- **Step 3: ledger commit** `chore: P0-1 exit — gates green, status synced`

## Review Focus (per-step pinning recap)

- Task 4 pins: duplicate-Request idempotency, priority-jump ordering, no-op generation stability, MediaDown-immediate semantics, mode-switch directions, preempt equality rule — each mapped above.
- Task 3 pins: snake_case tags, closed DenyReason rejection, role-scoped snapshot shapes.
- Task 2 pins: feature mutual exclusion compile assert; mac stub path.
- Server-domain behaviors deliberately OUT of this plan (plan-3): grace timer, AlreadyJoined enforcement, TokenRefresh push timing, /healthz real handler beyond stub.
