# Proposal: native-build-env-hardening

**Date**: 2026-10-27
**Status**: DRAFT — pending user adjudication
**Origin**: live debugging session 2026-10-27 (IDE `talkservo-deploy-build` failure chain, three nested root causes, all measured)

## What

Close the gap between "build works in an activated pixi shell" and "build works from any
process that is not a human in an activated shell" — the IDE button flow, ad-hoc CI-ish
local automation, and any future MediaServo consumer that inherits this graph. Three
independent mechanisms landed as one change:

1. **Live-lane env injection**: `talkservo-server-live`'s `_laneenv` gains
   `CC`/`CXX`/`LD_LIBRARY_PATH` (conda clang toolchain + conda libstdc++ runtime dir),
   following the lane's own existing precedent (`CARGO_TARGET_*_LINKER` pin +
   `NO_PROXY=*`), which in turn follows MediaServo's aarch64-lane precedent
   (MediaServoCrates.cmake:68, "the command wrapper strips host CC/CXX").
2. **Worker source-cache restore**: a small `scripts/worker-cache-restore.sh` +
   pixi task `worker-cache-restore` that re-seeds meson's `subprojects/packagecache`
   from a persistent staging dir (`~/.cache/talkservo/meson-packagecache/`, sha256-verified)
   before a cold live build, because mediasoup-sys's build.rs deletes the cache after every
   SUCCESSFUL build (cargo-publish hygiene, untouchable), and meson's github/wrapdb sources
   are unreachable from a bare-CN network without proxy.
3. **IDE entry surface**: `CMakePresets.json` (default/stub/live three-preset shape) +
   `.vscode/settings.example.json` (cmakePath pinned to the pixi env's cmake, presets
   wired) — the MediaServo pattern, so "which shell did you start VSCode from" stops
   being an undocumented contract.

## Why

The 2026-10-27 failure chain, measured end to end:

| # | Layer | Symptom | Root cause |
|---|---|---|---|
| 1 | cargo downloads | 4×30s zero-byte stalls per dependency, 8-min hang | crates.io bare-direct unreachable; no mirror configured (`~/.cargo/config.toml` absent); IDE process had no proxy vars |
| 2 | meson wrap downloads | `libsrtp ... is the internet available?` | github.com/wrapdb bare-direct unreachable; cargo mirror does not cover meson subproject fetches; packagecache cold after every successful build (build.rs cleanup) |
| 3 | C++ toolchain | `requires` only available with -fconcepts (gcc 9.4); then flatc `GLIBCXX_3.4.32 not found` | PolyOrch's wrapper strips host CC/CXX by design (PIT-14); activation.env (CC=conda clang, CXX=clang++-23, LD_LIBRARY_PATH=$CONDA_PREFIX/lib) never reaches the build because activation is a human act |

Fix #1 is host-user config (already landed outside this change: `~/.cargo/config.toml`
with rsproxy mirror). Fixes #2/#3 are repo-side and this proposal's body.

Why it matters beyond this host: D11 says Linux-native-first, Docker is a hand-run parity
image (D26 verification topology = one dev host). A native build that requires an activated
human shell makes the IDE button a lie for anyone whose VSCode came from a bare terminal —
which is the default. The same three mechanisms are exactly what MediaServo's Docker-first
posture hides; this project is where they get proven, then flow back.

## Scope

**In scope**
- `cmake/TalkServoCrates.cmake`: extend `_laneenv` for live lanes (CC/CXX/LD_LIBRARY_PATH)
- `scripts/worker-cache-restore.sh` (new) + `pixi.toml` task
- `CMakePresets.json` (new) + `.vscode/settings.example.json` (new)
- Docs: modules/09 (§3 activation posture + §4 script inventory), reference/README row if needed
- Verification: bare-env (deactivated) configure + live build + deploy staging, recorded
  as commands a human runs (C5 discipline — no hosted-CI claims)

**Out of scope**
- PolyOrch itself (supplier stays untouched; the injection rides `polyorch_rust_set_env_vars`,
  an existing API surface — same as the LINKER pin)
- mediasoup-sys build.rs cache-cleanup behavior (upstream, cargo-publish hygiene)
- Host-user network config (cargo mirror, proxy) — documented in modules/09, not automated
- MediaServo porting work (separate future change in that repo; this one proves the pattern)
- stub lanes (they don't compile the C++ worker; nothing to inject)

## Layers Affected

- HAL Core: no
- talkservo crates: no (source untouched)
- SFU host: no source change; **live lane build recipe changes** (crates/talkservo-sfu builds natively today, unchanged)
- Web SPA: no (web lane untouched; presets reference the existing graph targets)
- Build graph (cmake/): **yes — primary surface**
- pixi.toml: **yes** (one task)
- scripts/: **yes** (one new script)
- IDE/docs: **yes**

## Transports Affected

- talkservo (mediasoup lane): **yes — build recipe only, zero wire/runtime change**
- stub lane: no

## Existing Specs

- `docs/modules/09-dev-toolchain.md` — §3 pixi env posture (activation.env is where the
  toolchain tuning lives), §4 scripts. This change extends both.
- `docs/architecture.md` §4 — verification topology rows (D26). Bare-env build verification
  adds a row-level claim: the live lane builds without an activated shell.
- `.agents/memorys/pitfalls.md` PIT-14 (host-leak strip — the reason naive env setting
  doesn't work), PIT-36/37/38 (lane/gate traps this change must not repeat)

## New Specs Needed

- None as separate files. modules/09 §3 gains a subsection; pitfalls gains one entry
  (the flatc GLIBCXX / stripped-CC pair, filed at implementation time per lesson-memory).

## Risks

- **Env injection vs PIT-14 strip ordering**: injection must land AFTER the strip in the
  composed `cmake -E env` prefix. PolyOrch documents strip-then-assign semantics
  (PolyOrchRustHelpers.cmake:210-211, "an assignment placed AFTER --unset wins", cmake 4.4.3
  measured) and the existing LINKER pin already rides it — risk low, but the verify step
  must prove CC actually reaches meson (log grep for clang++-23 in meson-log.txt compiler lines).
- **Packagecache race**: two concurrent live builds would contend on meson's wrap lock
  (`.wraplock`). Mitigation: script is documented as pre-build step, single-invocation;
  build.rs already serializes per build-dir.
- **Staging dir is host-local** (`~/.cache/talkservo/`): a fresh machine starts cold and
  needs the restore script run once WITH network (proxy) to fill staging. Documented as
  one-time bootstrap, same tier as `pixi install`.
- **CMakePresets drift**: presets must name targets/options that exist (stub/live handles).
  Verified by configure + a `--list-presets` round-trip at implementation time.

## Success Criteria

1. `env -i HOME=$HOME PATH=/usr/bin:/bin` (a deliberately hostile, non-activated shell)
   → `cmake --build build --target talkservo-deploy-build` completes and stages
   `out/live/bin/talkservo-server`.
2. `pixi run worker-cache-restore` re-seeds packagecache from staging; live build after
   `rm -rf out/cargo-target/live` + cache wipe succeeds bare-network once staging exists.
3. VSCode opened with `.vscode/settings.example.json` copied to `settings.json` configures
   and builds the `live` preset without any shell activation (goal-state; acceptable to
   land as presets verified from CLI first).
4. `pixi run gate` stays green (ledger gates + C4/C5/C6 checks — doc claims name real commands).
5. modules/09 records the new posture; no C1/C4/C5/C6 violations (gate-enforced).

## References

- Failure-chain evidence session 2026-10-27 (this conversation): VSCode environ (pid 48631,
  no proxy), direct-curl 000 timeouts, meson-log GLIBCXX lines, successful 2m20s build with
  full activation.env equivalent.
- MediaServo precedents: `.refinfo/MediaServo/pixi.toml` (server-native tasks, unset-MESON
  trick), `cmake/MediaServoCrates.cmake:68` (CC/CXX inject into a stripped lane),
  `CMakePresets.json` + `.vscode/settings.example.json` (IDE face).
- PolyOrch docs: PolyOrchPixiHelpers.cmake header ("It cannot activate the caller's shell"),
  PolyOrchRustHelpers.cmake:191-211 (strip + after-strip assignment semantics).
- pixi.toml [activation.env] comments (flatc GLIBCXX note — the runtime half of root cause #3).
