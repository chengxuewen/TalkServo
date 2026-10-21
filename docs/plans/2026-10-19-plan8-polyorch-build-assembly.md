# Plan-8: PolyOrch build assembly — CMake total graph, scripts as thin wrappers

> **STATUS: P1-P4 LANDED 2026-10-19 (plan-8 complete; fresh-clone drill green; CI switched). Wave-1 plan-7 unaffected.**
> Rulings locked 2026-10-19 (D25: adopt-now / blueprint v2 phased / submodule embed).
> Implementation not started; commit ccffabc holds the spec, this revision is pending user approval.
> Sibling design truth: `docs/modules/09-dev-toolchain.md` (pixi posture; §PolyOrch pointer).
> Evidence base: `.refinfo/PolyOrch` (v0.3.0 Apache-2.0, upstream gitee.com/chengxuewen/PolyOrch),
> `.refinfo/VisiaEngine` (D39 embed shim, T-B/B1/G4 lanes), `.refinfo/ModuKit` (D66 same shape).
> User walkthrough 2026-10-19: scope C (blueprint-now, phased-light) / embed A (submodule
> pin-SHA + escape hatch) / blueprint v2 (full graph incl. install+package+live-e2e-label;
> runtime verbs OUTSIDE the graph as red lines).

## 0. What and why

PolyOrch is a CMake adapter layer for polyglot monorepos: cargo lanes, Node lanes, ctest
bridge, gate machinery — one build graph, native toolchains kept. TalkServo today is
pixi-tasks-only (12 `[tasks]`) with zero CMake surface; the Beta horizon (D8 bindings/,
npm/pip packaging, Qt/C/C++/wasm faces) is exactly the scenario PolyOrch serves, already
proven on two sibling repos. Decision: adopt now, phased, with the blueprint settled in full.

## 1a. Pre-flight (verified on dev host 2026-10-19)

- **GAP-1 cmake absent**: `command -v cmake` empty on host AND in the pixi env (pixi.toml
  has ninja/meson/clang, no cmake). PolyOrch floor is cmake >=3.25 (D39). P1.0 adds
  `cmake` (conda-forge) to the pixi host dependencies — an ENVIRONMENT change, allowed by
  D25 (pixi stays env single-source). CI parity job reuses the same lockfile.
- **GAP-2 pixi.sh ownership**: `polyorch_pixi_setup(COPY_SCRIPTS)` re-installs PolyOrch's
  own pixi.sh into the host repo (sister note: managed regions are committed). Today's
  `pixi.sh` is ours (bootstrap/D11 lineage). Decision at P1.0: adopt PolyOrch-managed
  region semantics for pixi.sh (sister-proven) OR keep ours and call `polyorch_pixi_setup`
  without COPY_SCRIPTS — P1.0 records the choice in modules/09 before any lane lands.
- `.gitignore` PolyOrch line removed by user 2026-10-19 (submodule path unshadowed) — OK.
- gitee reachability is probed at P1.1 (`git ls-remote`); failure = escape-hatch path
  (local checkout of .refinfo/PolyOrch promoted to a clone), recorded, not a blocker.

## 1b. Embed shape (adjudicated)

- `git submodule add https://gitee.com/chengxuewen/PolyOrch.git PolyOrch`, pinned at the
  0.3.0-tested SHA. Fresh clone: `git submodule update --init --recursive`.
- Escape hatch `-DTALKSERVO_POLYORCH_DIR=<path>` (superbuild/offline), verbatim sister pattern.
- `cmake/TalkServoPolyOrch.cmake` shim: sentinel first-wins reuse (D39 semantics), version
  floor probe (missing command => FATAL naming the fix), `polyorch_rust_setup(FROM pixi)`
  translation of the pixi environment.
- pixi stays environment single-source (D5 unchanged): conda rust/clang/meson/node env;
  CMake consumes it, never replaces it.

## 2. Blueprint v2 (the full graph — end state, all faces lit)

| Face | Content | Mechanism |
|---|---|---|
| Rust lanes | core / sfu / server, each × `stub-media` and `sfu-mediasoup,embedded-web` feature configs | PolyOrch cargo helpers (VisiaEngine T-B shape) |
| Web lane | `web/` build / vitest / tsc | PolyOrch Node helpers (ModuKit precedent) |
| e2e | Playwright stub lane = default ctest; live lane (mediasoup/coturn/netem) = label-gated `ctest -L e2e-live` | ctest bridge + fixtures reuse global-setup |
| gates | C1/C2/C3/link-integrity + fmt/clippy/deny + `gate.sh` managed regions | PolyOrch gate machinery |
| package | install tree + versioned tar (`--strip-components` contract preserved) | CMake install (VisiaEngine install-tree precedent) |
| scripts | `talkservo.sh build/package/doctor-build` become thin forwarders to `cmake --build/--install` + `ctest -L` | thin wrapper rule |

### Red lines (graph boundary — adjudicated v2)

1. **Runtime verbs NEVER enter the graph**: `init/start/stop/restart/status/logs/monit/ps`
   (oxmgr facade, D19) and `up/down` (compose) stay script+facade territory. Build graph
   ends at "installable artifact produced"; instance lifecycle is not a build concern.
2. **mediasoup C++ worker stays build.rs-owned** (crate build chain via meson/ninja); CMake
   invokes `cargo build` through lanes — a second C++ configure path is FORBIDDEN.
3. **bindings/ gets wiring slots, not lanes**: Beta (D8) may add Qt/C/C++/wasm faces — the
   graph reserves `add_subdirectory` comment positions only (no speculative scaffolding).
4. `packages/client` npm publish flow: graph owns compile/test faces only.

## 3. Construction schedule (the "C" ruling: blueprint fixed, lights phased)

### P1 detailed task list (the slice that unlocks the rest)

- [x] 1.0 env prep: add `cmake >=3.25` to pixi.toml host dependencies; `pixi lock`;
      decide pixi.sh COPY_SCRIPTS posture (record in modules/09 pointer section)
- [x] 1.1 submodule: probe `git ls-remote https://gitee.com/chengxuewen/PolyOrch.git`;
      `git submodule add` pinned at the 0.3.0-tested SHA; fresh-clone drill:
      `git submodule update --init --recursive`
- [x] 1.2 shim: `cmake/TalkServoPolyOrch.cmake` — sentinel first-wins reuse, version-floor
      probe (missing command => FATAL with the fix), `polyorch_rust_setup(FROM pixi)`
      translation (VisiaEnginePolyOrch.cmake as the reference, D39 contract)
- [x] 1.3 root `CMakeLists.txt` (thin): cmake_minimum 3.25; project(talkservo ...)
      LANGUAGES NONE; include shim; ensure_polyorch + rust setup; NO CXX identity
      (ModuKit note: CXX before embed only if the PlatformSupport contract needs it —
      TalkServo has no C++ targets at graph level, verify the first-red)
- [x] 1.4 core lane: talkservo-core through PolyOrch cargo helpers -> ctest label `rust`;
      stub-feature lane only at P1 (mediasoup lane waits for P2 to keep configure cheap)
- [x] 1.5 discoverability: pixi task `cmake-probe = "cmake -B build -G Ninja"` style entry;
      README quickstart one-liner (sisters' bootstrap->pixi->cmake flow)
- [x] 1.6 docs: AGENTS.md COMMANDS gains the cmake entry (P1 obligation, §4)
- [x] 1.7 exit: Linux green = configure + build + `ctest -L rust` runs core's 39 tests
      through PolyOrch; existing flows untouched (pixi test / web vitest / stub e2e still
      pass); commit `feat(build): P1 PolyOrch skeleton — submodule, shim, core lane (plan-8)`

### P2-P4 (task-level detail lands at each phase start; scope fixed here)

- [x] **P2 lanes** LANDED: five cargo lanes (core/sfu/server-lib/server-bin stub config +
  server-mediasoup artifact, feature column + per-config target dirs) + web Node lane; pixi
  build-verbs -> forwarders (names kept, CI untouched). Exit GREEN: ctest -L "rust|web" 4/4
  (39 core / 1 sfu / 79 server-stub / vitest 8) + vite build 8.9s through the graph.
- [x] **P3 gates+e2e** LANDED: scripts/gate.sh (C1-C3 at the time, now C1-C4 — see C4 below — embedded VERBATIM from conventions.md
  via scripts/gate-gen.py - ledger Check lines upgraded to executable PASS/FAIL blocks) +
  gate-fmt/clippy/deny + e2e-stub/e2e-live labels riding _polyorch_rust_command.
  Exit GREEN: ctest -L "rust|web|gate|e2e-stub" 9/9 (deny honest-skip). Forensics side-quest:
  a globalSetup/workers config line lost in the test-honesty batch made every ctest e2e run
  bare - restored, talk.spec cross-client assertions executed for the first time (field-page
  check rebased onto tally-ring locator), and the initial "node22 env-snapshot" theory was
  corrected in-place to the true single cause (missing globalSetup).
- [x] **P4 package+CI** LANDED: TalkServoInstall components (bin / dist) + talkservo.sh
  build/package thin forwarders + CI job set switched (graph + e2e via ctest labels; check
  matrix, test-mediasoup, audit stay cargo-direct). Parity: tar member list identical,
  text-file hashes identical, oxmgr identical; the server BINARY differs by design - the
  old cmd_build silently built in the pixi DEFAULT env while the graph (and everything
  else) uses -e dev; unification is the fix, and cargo path-embedding + build-id policy
  make byte-parity unattainable across dirs. Functional smoke on the graph binary
  (healthz/room-page/admin-401) + fresh-clone drill 4/4 (clone -> submodule init from
  gitee -> npm ci x2 -> web lane -> ctest -L "rust|web"). Lane-ordering guard added:
  embedded-web mediators depend on talkservo-web-build (rust-embed bakes web/dist at
  compile time - fresh clones hit E0599 Assets::get otherwise).

Interleave: plan-8 and plan-7 are orthogonal — plan-8 P1 lands before/between/after any
plan-7 slice; plan-7 gates are entry-agnostic grep/cargo invocations either way (pixi
native until P2, forwarders P2-P4, cmake sole build entry after P4 CI swap).


## 4. Docs & ledger obligations (per phase, plan-5 §8 style)

- **now (D25)**: decisions.md D25 appended; modules/09 pointer section; docs/README plan
  row; status.md decision table + phase line.
- **P1**: AGENTS.md COMMANDS gains the cmake entry (pixi rows still true).
- **P2**: modules/09 §pixi.toml shape section rewritten to "environment + forwarders";
  AGENTS.md commands swap build verbs to the forwarders.
- **P3**: `.agents/rules/common/{docker,platform}.md` untouched (runtime/policy, not build).
- **P4**: CI verb swap lands here — ci-cd-automation skill refreshes its pixi-task table to
  cmake/ctest verbs; server-api/config handbook unaffected; talkservo.sh header comment
  updates (build: now a forwarder); modules/06 backup-set line gains PolyOrch note (submodule
  SHA is part of reproducibility, `git ls-files` verifies, not file-copy).

## 5. Verification & acceptance (plan-8 exit)

- Fresh-clone reproducibility: clone -> submodule init -> bootstrap -> `cmake --build` ->
  `ctest -L "gate || rust || e2e-stub"` all green on Linux (CI parity job).
- Parity proof: artifact tar contents diff = empty vs today; every existing pixi build task
  ends as a thin forwarder or an environment verb — no dead task names in pixi.toml.
- Sister-contract checks: sentinel reuse path tested via `-DTALKSERVO_POLYORCH_DIR` probe;
  version-floor FATAL wording reviewed once.

## 6. Adjudication record (user-adjudicated, this session)

| # | question | decision |
|---|----------|----------|
| 1 | adopt PolyOrch? | A — adopt now (not Beta-defer) |
| 2 | end-state graph scope | C — blueprint settled now, lights phased; then user widened to **v2 full graph** (install+package+live-e2e-label in; runtime verbs out — red lines) |
| 3 | embed method | A — git submodule pin-SHA + `TALKSERVO_POLYORCH_DIR` escape hatch |

Open items: none blocking; `.gitmodules` URL is gitee (mirror policy if ever needed =
record when network pain is real, not pre-emptively).
