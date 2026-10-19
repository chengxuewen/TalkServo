# Plan-8: PolyOrch build assembly — CMake total graph, scripts as thin wrappers

> **STATUS: APPROVED 2026-10-19 (D25) — implementation not started.**
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

## 1. Embed shape (adjudicated)

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

| Phase | Deliverables | Exit gate |
|---|---|---|
| **P1 skeleton** | submodule + `.gitmodules` + shim + thin root CMakeLists + **core lane** into ctest + `pixi run cmake-probe` task (discoverability) | Linux: `cmake --build` configures + `ctest -L rust` runs core tests through PolyOrch; zero changes to existing pixi/web daily flow |
| **P2 lanes** | sfu+server lanes × dual feature configs; web lane (build/vitest/tsc); **pixi build-verbs become thin forwarders to cmake** (names preserved, CI untouched) | full daily flow reachable via cmake/ctest verbs; pixi task names still work (= forwarders, env verbs kept) |
| **P3 gates+e2e** | gate face ctest-ified (`-L gate`); Playwright stub lane `-L e2e-stub` default, live `-L e2e-live` label-gated | one verb table covers build/test/gate; live lane keeps env flags TS_E2E_FEATURES/TS_COTURN semantics |
| **P4 package+CI** | install tree + tar target; `talkservo.sh build/package` thin; CI workflow verbs = `cmake --build` + `ctest -L` | artifact parity vs today (tar contents diff-tested); CI matrix unchanged platforms |

Interleave note: plan-8 and plan-7 (wave-1 trust&control) are orthogonal workstreams —
plan-8 P1 can land before, after, or between plan-7 slices; plan-7 slices' gates work under
either entry (pixi native until P2, forwarders P2-P4, cmake sole build entry after P4 CI swap — plan-7 gates are entry-agnostic grep/cargo invocations either way).

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
