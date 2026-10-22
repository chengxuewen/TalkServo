# Plan-10: Graph target vocabulary — one handle per (package, config), derived consumers, live test in-graph

> **STATUS: PROPOSED — awaiting user review. No code changed.** Not approved for execution.
> Argues from: D25 (PolyOrch graph, cargo keeps authority), D26 (verification is local — the
> `gate-docs` ctest entry IS the pre-push hook's script), `docs/modules/09-dev-toolchain.md`.
> Evidence base measured on this host 2026-10-22 (see §1, all commands re-runnable).
> Open forks are in §4 — the plan cannot execute until the user answers them.

> **EXECUTION STATUS 2026-10-22: T1-T4 + T6 + T7 LANDED** (commits `b07be4b` plan, `7b95b05`
> vocabulary+derivation, `403c9f7` live test verb, `c1334f9` T6 reverted-with-numbers,
> `5e3c91e` gate-clippy repair, `799c386` docs/ledger). F1=B, F2=O2, F3=C1-after-measurement,
> F4 untouched. **T5 partially run**: `pixi run test` 4/4, `pixi run e2e` 1/1, live lane 1/1
> under `TS_LANE_LIVE=1`; NOT run to completion: the fresh-clone drill and a cold
> `pixi run gate` (each tens of minutes after `./target` and mediasoup-sys output were cleared
> during the T6 experiment). Steps below are left as written; the two deviations are folded
> into Task 4 Step 1 (gate script + `mediasoup` label) and the Executed-findings log.

**Goal:** Replace the handle names that describe the same deliverable twice (`talkservo-server` lib + `talkservo-server-bin`) with one handle per (package, configuration), put every name on a single `<object>-<verb>` axis, delete the four hand-written consumer lists that can silently drift, and bring the mediasoup backend's test verb inside the graph.

**Architecture:** Cargo stays the build engine; this graph only names and drives it (plan-8 red line). A lane is declared once through a new `talkservo_rust_lane()` named-argument function that composes the PolyOrch build/test/clean handles and registers itself into global properties; the web-ordering edges, the install handle choice and the aggregate all read those properties instead of repeating names.

**Tech Stack:** CMake ≥3.25 + Ninja, PolyOrch helper surface (`polyorch_rust_build/_test/_clean`, `_polyorch_rust_command`), cargo, ctest labels, pixi tasks as thin forwarders.

**Spec:** this file. There is no separate design doc; §1 is the measurement the design argues from, and `docs/modules/09-dev-toolchain.md` §"lane vocabulary" (Task 7) becomes the durable rule.

## Global Constraints

- `crates/talkservo-{core,sfu,server}/Cargo.toml` features unchanged; `default = ["sfu-mediasoup"]` stays (D5/D16 dual-gate: exactly one backend per compile unit is cfg-asserted).
- Zero compilation rules in the graph: no `add_library`/`add_executable` for Rust code, no `add_subdirectory(crates/...)` (plan-8 red line 2).
- Runtime verbs (oxmgr facade, compose, Caddy) stay outside the graph (plan-8 red line 1).
- pixi remains the environment single source; the graph consumes it via `TALKSERVO_USE_PIXI` (D25).
- `enable_testing()` before every `add_subdirectory` (measured lesson, `CMakeLists.txt:21`).
- Project version stays read from `Cargo.toml` (`CMakeLists.txt:10-16`); no literal copies.
- Artifacts in English (C1); doc-tier rules (C2); no single-letter user-facing names (C3 — applies to target names, which a human types); ledger-index rows updated in the same commit (C4); no hosted-CI wording (C5/D26).
- Linker pin stays `cc` (not `clang++-23`) and `NO_PROXY=*` stays, per `cmake/TalkServoCrates.cmake:69-87` — a lane that loses either dies on `__libc_single_threaded` or on the MITM-pypi TLS break.

---

## 1. Measured basis (all re-runnable)

| Fact | Evidence |
|---|---|
| The graph already passes target selectors: lib handle = `cargo build -p talkservo-server --lib`, bin handle = `… --bin talkservo-server` | `out/cmake-build/build.ninja` COMMAND lines (`--lib` / `--bin talkservo-server`) |
| One `--bin` invocation compiles **both** units (lib first, then the bin linking that rlib) → the `--lib` handle saves no compilation | `cargo build -p talkservo-server --no-default-features --features stub-media,embedded-web --bin talkservo-server -v` after touching `src/lib.rs` → two `Running \`rustc --crate-type lib\`` + `--crate-type bin\`` lines in one call |
| A whole-package invocation when fresh == the no-op floor (≈0.25s); dirty whole-package ≈ dirty `--lib` (2.13s vs 2.66s) | timed runs 2026-10-22 (note: measured in `./target`; magnitudes only — see §1 last row) |
| The helper forces exactly one artifact kind per handle, and requires `CRATE` | `.refinfo/PolyOrch/cmake/PolyOrchRustHelpers.cmake:282` ("pick exactly one of BINARY, STATIC, SHARED, EXAMPLE, LIB"), signature at `:233` |
| `polyorch_rust_test` needs only `TARGET` — it does not require a LIB-kind handle | `:1869-1876` |
| Gate commands get **no** `--target-dir` → `gate-fmt`/`gate-clippy` compile into cargo's default `./target`, while lanes compile into `out/cargo-target*` = two caches for one workspace | `PolyOrchRustHelpers.cmake:99` ("`--target-dir` is NOT included"), `:503/:1888` (lanes do pass it); `./target` measured at 33 GB |
| PolyOrch's own comment references a `polyorch-rust-all` aggregate that does not exist in this version | `:621` comment only; `grep -c polyorch-rust-all out/cmake-build/build.ninja` = 0 |
| Top-level `all` builds **nothing** of TalkServo's (only the embedded PolyOrch dir) → there is no default goal today | `out/cmake-build/build.ninja:423` `build all: phony _polyorch/all` |
| Mainstream layout: lib+bin in one package is the norm; two *build units* are made by two *packages*; CI invokes per package (`-p`), `--bin` only to pick a specific/gated bin | cargo, rust-analyzer (`crates/rust-analyzer/Cargo.toml` `[lib]`+`[[bin]]`, CI `cargo build -p rust-analyzer`), deno (`cli/Cargo.toml` lib+bin, CI `--bin deno --bin denort`), vector (same package + `required-features` gated extra bin), zed / nushell (bin-only packages), bevy/tokio (lib-only) |
| `required-features` on the bin pattern cannot be copied here: our single bin must build under BOTH backends (`e2e-stub` runs the stub binary) | `crates/talkservo-server/Cargo.toml:13-15` (`[[bin]]` has no `required-features`), `TalkServoGates.cmake:61-64` |
| Manual cargo calls without `--target-dir` silently fork a second cache | this session: several probe builds created/filled `./target` while the graph uses `out/cargo-target` |

## 2. The target set after this plan (review this table)

Objects carry a config suffix **only** where the same package has more than one in-graph config (measured: only `talkservo-server` does). `stub|live` is the same word already used by `TALKSERVO_PACKAGE_LANE`, `talkservo.sh --lane`, `ctest -L e2e-stub/e2e-live`, `TS_E2E_FEATURES=live`.

| # | target | verb | drives | cache group | ctest entry + labels |
|---|---|---|---|---|---|
| 1 | `talkservo-core-build` | build | `cargo build -p talkservo-core` (defaults, LIB) | `out/cargo-target/stub` | — |
| 2 | `talkservo-core-test` | test | `cargo test -p talkservo-core` | same | `talkservo-core-test` — `rust;headless` |
| 3 | `talkservo-core-clean` | clean | `cargo clean -p talkservo-core --target-dir out/cargo-target/stub` | — | — |
| 4 | `talkservo-sfu-build` | build | `… -p talkservo-sfu --lib`, `stub-media` | `out/cargo-target/stub` | — |
| 5 | `talkservo-sfu-test` | test | `… test -p talkservo-sfu` | same | `talkservo-sfu-test` — `rust;headless` |
| 6 | `talkservo-sfu-clean` | clean | `cargo clean -p talkservo-sfu …` | — | — |
| 7 | `talkservo-server-stub-build` | build | `… -p talkservo-server --bin talkservo-server`, `--no-default-features --features stub-media,embedded-web` | `out/cargo-target/stub` | — |
| 8 | `talkservo-server-stub-test` | test | `… test -p talkservo-server` (88 lib + integration) | same | `talkservo-server-stub-test` — `rust;headless` |
| 9 | `talkservo-server-stub-clean` | clean | `cargo clean -p talkservo-server --target-dir out/cargo-target/stub` | — | — |
| 10 | `talkservo-server-live-build` | build | same `--bin`, `--no-default-features --features sfu-mediasoup,embedded-web` (the only target that compiles the C++ worker) | `out/cargo-target/live` | — |
| 11 | `talkservo-server-live-test` | test | `cargo test -p talkservo-server --no-default-features --features sfu-mediasoup,embedded-web`; **skips (77) unless `TS_LANE_LIVE=1`** | same | `talkservo-server-live-test` — `mediasoup;headless;slow` |
| 12 | `talkservo-server-live-clean` | clean | `cargo clean -p talkservo-server --target-dir out/cargo-target/live` | — | — |
| 13 | `talkservo-web-build` | build | `npm run build` → `web/dist` (feeds `embedded-web` at compile time) | npm | — |
| 14 | `talkservo-web-test` | test | `npm run test` (vitest) | npm | `web-test` — `web;headless` |
| 15 | `talkservo-web-clean` | clean | `cmake -E rm -rf web/dist` | — | — |
| 16 | `talkservo-lanes-stub-build` | scope | aggregate DEPENDS 1,4,7,13 (never the live lane) | — | — |
| 17 | `talkservo-lanes-clean` | scope | `cmake -E rm -rf out/cargo-target` (both groups) | — | — |

Unchanged names/semantics: `gate-docs` (= `scripts/gate.sh`, the same script the pre-push hook runs), `gate-fmt`, `gate-clippy`, `gate-deny`, `e2e-stub`, `e2e-live`, install components `bin`/`dist`, `cmake --install`, CMake built-ins.
ctest totals after: **11 entries**; `-L rust` = 3 (unchanged meaning: stub side), `-L mediasoup` = 1 (new), `-L web` = 1, `-L gate` = 4, `-L e2e-stub` = 1, `-L e2e-live` = 1.
Deleted targets: `talkservo-server-build` (lib handle — its rlib had no consumer in the graph), `talkservo-server-bin-build` (-> #7), `talkservo-server-mediasoup-build` (-> #10), `lanes-mediasoup-clean` (folded into #17). Renamed: `lanes-clean` -> `talkservo-lanes-clean` (same action, now project-prefixed).

## 3. Before → after, and the rule that gets written down

```
talkservo-core            (lib)              → talkservo-core-*                      # unchanged
talkservo-sfu             (lib, stub-media)  → talkservo-sfu-*                       # unchanged name
talkservo-server          (lib,  stub cfg)   →┐
talkservo-server-bin      (bin,  stub cfg)   →┘ talkservo-server-stub-*              # 3 handles → 2 per backend
talkservo-server-mediasoup(bin,  live cfg)   → talkservo-server-live-* (+ -test)
lanes-clean + lanes-mediasoup-clean                -> talkservo-lanes-clean (parent dir)
(none)                                       → talkservo-lanes-stub-build (aggregate), <handle>-clean per object
```

Rule added to `docs/modules/09-dev-toolchain.md`: every graph button is named `talkservo-<object>-<verb>`, verb one of `build` / `test` / `clean`, object = a lane `<crate>[-<config>]` or a scope `lanes[-<config>]`, config one of {omitted, `stub`, `live`}; a lane carries the config suffix only when that package has more than one in-graph configuration. One handle per (package, config), tracking that config's delivered artifact: `BINARY` when the package has a bin, else `LIB`; a second handle for the same package exists only when something in the graph actually consumes the other artifact (the clause reserved for the future C-ABI staticlib, D8). The project prefix is mandatory, not decorative: target names are global to the CMake project and this graph is designed to be embedded in a superbuild (`cmake/TalkServoPolyOrch.cmake:12-16`, first-wins sentinel), where a bare `lanes-clean` would collide with any sibling repo adopting the same PolyOrch pattern; upstream names its own aggregate `polyorch-rust-all` (`PolyOrchRustHelpers.cmake:621` - named in a comment, the target is not created in the pinned version), and one prefix also makes the ninja target list filtered on the `talkservo-` prefix the complete build surface.

## 4. Open forks — the user must answer before Task 1

| Fork | Options | My recommendation |
|---|---|---|
| **F1 default goal** | A: graph keeps no default target (bare `cmake --build` builds nothing, as today) / B: `talkservo-lanes-stub-build` gets the `ALL` keyword (bare `cmake --build` = the daily path, worker untouched) | **B** — a build system with a silent empty default is the "green from a mechanism that never ran" family (PIT-27); B costs nothing on the live lane because the aggregate excludes it |
| **F2 per-object clean** | O1: only `talkservo-lanes-clean`, document `cargo clean -p <pkg> --target-dir out/cargo-target/stub` as the surgical path / O2: emit `<handle>-clean` for every object (rows 3,6,9,12,15) | **O2** — it puts `clean` on the same axis for every object (the objection that started this plan) and each name's scope is honest (`cargo clean -p` per package, `rm -rf` per cache group). Cost: +5 names |
| **F3 gate cache consolidation** | C1: leave `gate-fmt`/`gate-clippy` on `./target` / C2: give `gate-clippy` `--target-dir out/cargo-target/live` so its worker/build-script outputs are shared with lane #10 | **C2, but only if measured** (Task 6 runs the timing gate and keeps C1 if it doesn't help) |
| **F4 PolyOrch gitlink** | P1: verify the whole graph on the checked-out `879856a` and commit it as the new pin / P2: `git submodule update PolyOrch` back to the recorded `5945f2a` | depends on Task 8's green run; if green, **P1** (a pin nobody builds against is not a pin) |

---

## Task 1: `talkservo_rust_lane()` — declare lanes with named args, register them

**Files:**
- Modify: `cmake/TalkServoCrates.cmake` (replace the `@`-string table `:22-27` and the body of `talkservo_crates()` `:39-114`; keep the env/linker pins `:69-87` intact)

**Interfaces (produces — later tasks read exactly these):**
- `talkservo_rust_lane(HANDLE <h> PACKAGE <p> KIND LIB|BINARY [FEATURES <f>] [CACHE stub|live] [TEST ON|OFF] [TEST_LABELS "<labels>"] [CLEAN ON|OFF])`
- global property `TALKSERVO_LANE_HANDLES` — every registered `<handle>;<kind>;<cache>` triple
- global property `TALKSERVO_EMBEDDED_WEB_HANDLES` — handles whose FEATURES contain `embedded-web`
- derived, never hand-written: `MANIFEST = <source>/crates/<PACKAGE>/Cargo.toml`, `CRATE = <PACKAGE with - → _>`, `BASE_DIR = <source>/out/cargo-target/<CACHE>`, ctest name `<HANDLE>-test`, clean target `<HANDLE>-clean`

- [ ] **Step 1: write the failing inventory check** — run `cmake -B out/cmake-build -G Ninja` (via `pixi run cmake-probe`) with the lane table left as-is and assert the new names are absent:
      `ninja -C out/cmake-build -t targets | grep -c '^talkservo-server-stub-build:'` → expected `0`
- [ ] **Step 2: implement `talkservo_rust_lane()`** and the 4 rust lane calls listed in §2 rows 1-12. No feature-flag semantics change: non-empty FEATURES ⇒ `polyorch_rust_set_features(NO_DEFAULT_FEATURES FEATURES …)`, empty ⇒ package defaults.
- [ ] **Step 3: reconfigure and prove the inventory**
      `~/.pixi/bin/pixi run -e dev cmake -B out/cmake-build -G Ninja` then
      inventory (this exact form was verified on 2026-10-22): `grep -oE '^build [A-Za-z0-9_.-]+: phony' out/cmake-build/build.ninja | awk '{print $2}' | grep -E -- '-(build|test|clean):$' | sort -u`;
      prefix ratchet: the same pipeline with `grep -v '^talkservo-'` appended, which must print nothing.
      **Command hazard, measured:** plain `ninja -C out/cmake-build -t targets` prints *nothing* from a shell without the pixi env on PATH (`command -v ninja` is empty), and a downstream grep then reports success on the empty stream - a silent false PASS, PIT-30's PATH-blindness family. Either use the `build.ninja` grep above or prefix the command with `~/.pixi/bin/pixi run -e dev`.
      Expected: exactly the 17 names of §2 (the 2 scope rows included), the prefix ratchet prints nothing (baseline: the same ratchet against today's graph prints exactly `lanes-clean:` and `lanes-mediasoup-clean:`, and the inventory lists 12 buttons today vs 17 after); **zero** hits for `talkservo-server-bin`, `talkservo-server-mediasoup`, `talkservo-server-build`, `lanes-clean`, `lanes-mediasoup-clean`.
- [ ] **Step 4: prove the lane still works** — `~/.pixi/bin/pixi run -e dev cmake --build out/cmake-build --target talkservo-server-stub-build` → exit 0 and `out/cargo-target/stub/debug/talkservo-server` exists.
- [ ] **Step 5: commit** `refactor(build): one lane handle per (package, config) via named-arg declaration (plan-10 T1)`

## Task 2: delete the four hand-written name lists (derive from the registry)

**Files:**
- Modify: `cmake/TalkServoWeb.cmake:27-31` (the `foreach(_dep talkservo-server talkservo-server-bin …)` list)
- Modify: `cmake/TalkServoInstall.cmake:17-21` (the if/else handle choice)
- Modify: `talkservo.sh:74-75` (the `--lane` → target-name mapping)
- Modify: `pixi.toml:32` (`build` enumerates six target names)

**Interfaces:** consumes `TALKSERVO_EMBEDDED_WEB_HANDLES`, `TALKSERVO_LANE_HANDLES`, `TALKSERVO_PACKAGE_LANE`.

- [ ] **Step 1: web ordering** — `foreach(_h ${TALKSERVO_EMBEDDED_WEB_HANDLES}) add_dependencies(${_h}-build talkservo-web-build) endforeach()`, replacing the literal list. Guard stays `if(TARGET …)` so `TALKSERVO_WEB=OFF` cannot break configure.
- [ ] **Step 2: install** — `set(_handle talkservo-server-${TALKSERVO_PACKAGE_LANE})`; keep the `stub|live` FATAL validation (`:14-16`).
- [ ] **Step 3: script + pixi** — `talkservo.sh`: `tgt=talkservo-server-${lane}-build` (one line); `pixi.toml`: `build = "cmake --build out/cmake-build --target talkservo-lanes-stub-build"` (per F1=B) or the four names (F1=A).
- [ ] **Step 4: prove no stale names survive** — 
      `grep -rnE 'talkservo-server-bin|talkservo-server-mediasoup|cargo-target-mediasoup' --exclude-dir=out --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules . | grep -v docs/plans/` → empty (historical plan files are exempt as working-tier records).
- [ ] **Step 5: prove both install lanes still stage** —
      `~/.pixi/bin/pixi run -e dev cmake --build out/cmake-build --target talkservo-server-stub-build && cmake --install out/cmake-build --prefix /tmp/opencode/ts-i-stub --component bin` → `bin/talkservo-server` exists; then reconfigure with `-DTALKSERVO_PACKAGE_LANE=live`, build `talkservo-server-live-build`, install to `/tmp/opencode/ts-i-live`, same expectation.
- [ ] **Step 6: prove the live install fails loudly, not silently** — after `talkservo-lanes-clean` (no worker ever built), reconfigure with `-DTALKSERVO_PACKAGE_LANE=live` and run `cmake --install out/cmake-build --prefix /tmp/opencode/ts-i-live2 --component bin` without building `talkservo-server-live-build` first. Expected: non-zero exit or a missing-artifact error, and **never** a stub exe staged under a `live` claim. If CMake installs nothing with exit 0, add a configure-time guard in `cmake/TalkServoInstall.cmake` (`if(NOT EXISTS <tracked artifact>) message(FATAL_ERROR …)`) as part of this task.
- [ ] **Step 7: commit** `refactor(build): derive web-ordering edges, install handle and daily target from the lane registry (plan-10 T2)`

## Task 3: aggregate + clean semantics

**Files:** Modify `cmake/TalkServoCrates.cmake` (aggregate; replace `:117-118`), `cmake/TalkServoWeb.cmake` (its `-clean` if F2=O2).

- [ ] **Step 1: aggregate** `add_custom_target(talkservo-lanes-stub-build [ALL] DEPENDS <every stub-cache-group handle>-build)` — built from `TALKSERVO_LANE_HANDLES` filtered on `;stub`, never hand-listed; `[ALL]` only if F1=B.
- [ ] **Step 2: scope clean** one `polyorch_rust_clean(NAME talkservo-lanes-clean BASE_DIR out/cargo-target …)` (parent of both groups), dropping `lanes-mediasoup-clean`.
- [ ] **Step 3: object cleans** (F2=O2) `add_custom_target(<handle>-clean COMMAND <_polyorch_rust_command> clean -p <PACKAGE> --target-dir <BASE_DIR>)` emitted inside the lane function when `CLEAN=ON`; web's is `cmake -E rm -rf web/dist`.
- [ ] **Step 4: prove clean semantics** — build `talkservo-server-stub-build`; run `talkservo-core-clean`; then `test -f out/cargo-target/stub/debug/talkservo-server && echo "exe survived core-clean"` (must survive) and the core rlib must be gone. Then `talkservo-lanes-clean` and assert `out/cargo-target` absent.
- [ ] **Step 5: prove the default goal** (F1=B) — `~/.pixi/bin/pixi run -e dev cmake --build out/cmake-build` builds web + 3 rust stub lanes and does **not** produce `out/cargo-target/live/*`.
- [ ] **Step 6: commit** `feat(build): stub-scope aggregate and per-object package-scoped clean targets (plan-10 T3)`

## Task 4: bring the live backend's test verb into the graph

**Files:** Modify `cmake/TalkServoCrates.cmake` (lane #11 `TEST=ON TEST_LABELS "mediasoup;slow"` + runtime gate), `pixi.toml:37`.

- [ ] **Step 1: register** `talkservo-server-live-test` with `COMMAND` = `sh -c "test \"\${TS_LANE_LIVE:-}\" = 1 || exit 77; <same argv as the -test button>"`, `SKIP_RETURN_CODE 77`. Mechanism copied from `TalkServoGates.cmake:66-69` (`e2e-live`) — no new knob vocabulary.
- [ ] **Step 2: forwarder** `test-sfu = "TS_LANE_LIVE=1 ctest --test-dir out/cmake-build -L mediasoup"`.
- [ ] **Step 3: prove the skip is honest** — `ctest --test-dir out/cmake-build -L mediasoup` unset env → `Skipped 1/1`; with `TS_LANE_LIVE=1` → `Passed` (expect worker build ≈4 min).
- [ ] **Step 4: prove the fast lane did not get slower** — `ctest --test-dir out/cmake-build -L "rust|web" -N` → exactly 4 entries (3 rust + 1 web); the live entry must not appear.
- [ ] **Step 5: commit** `test(build): mediasoup server lane gets an in-graph test verb behind TS_LANE_LIVE (plan-10 T4)`

## Task 5: full-lane regression battery

- [ ] **Step 1:** `~/.pixi/bin/pixi run test` → `100% tests passed out of 4` (3 rust + web).
- [ ] **Step 2:** `~/.pixi/bin/pixi run gate` → `out of 4` (gate-docs runs `scripts/gate.sh` = 6 `[PASS]`).
- [ ] **Step 3:** `~/.pixi/bin/pixi run e2e` → `out of 1` (stub-lane admin+floor rows incl. the push/degrade rows).
- [ ] **Step 4:** fresh-clone drill (manual, as plan-8 P4 ran it): `git clone <local bundle> /tmp/opencode/ts-drill && git -C … submodule update --init --recursive`, then `pixi run -e dev cmake -B out/cmake-build -G Ninja && pixi run -e dev ctest --test-dir out/cmake-build -L "rust|web|gate" -E e2e` → all pass, zero network beyond the pinned submodule.
- [ ] **Step 5:** commit nothing (verification only; report the four outputs in the commit message of Task 6 or 7).

## Task 6: cache consolidation experiment (only if F3=C2)

- [ ] **Step 1: measure baseline** — cold: `time ~/.pixi/bin/pixi run -e dev cargo clippy --workspace --all-targets -- -D warnings` into `./target`, and the same command with `--target-dir out/cargo-target/live` in a second fresh dir. Record both seconds and the `du -sh` of each dir.
- [ ] **Step 2: decide by the number** — keep C2 (share `out/cargo-target/live`) only if the shared dir's cold time ≤ the `./target` cold time (worker build-script output reuse is the point); otherwise keep C1, and write the measured numbers into `docs/modules/09` so nobody re-pays the experiment.
- [ ] **Step 3: if kept** — add `--target-dir` into the `gate-clippy` SUBCOMMAND argv in `cmake/TalkServoGates.cmake:32` and re-run `pixi run gate`.
- [ ] **Step 4: commit** `perf(build): share the live cache group with the clippy gate (plan-10 T6)` or `docs(build): record measured cache-split cost, keep gate on ./target (plan-10 T6)`.

## Task 7: documentation + ledger obligations

**Files:** Modify `docs/modules/09-dev-toolchain.md` (lane inventory, the §3 rule, the no-`--target-dir` warning, "graph has no default goal" or the new `ALL` behavior), `docs/README.md` (plans row for plan-10), `.agents/memorys/decisions.md` (**D27**: graph target vocabulary + one handle per package-config, with the measured basis), `.agents/memorys/status.md` (Decision Status table row D27 + Next Steps), `AGENTS.md` (Ranges row `D1-D27` **in the same commit**, C4 rule), `.github/workflows/ci.yml` (INERT banner is already there; update the verb comments only if labels changed).

- [ ] **Step 1:** write the rule line and the new inventory table into `docs/modules/09`; state there that `--lib`/`--bin` selectors are the helper's artifact-tracking choice, not a work-saving split (cite the `-v` evidence).
- [ ] **Step 2:** append D27 to `decisions.md` with a runnable **Check** (the §2 inventory one-liner) and the three flip conditions from §3.
- [ ] **Step 3:** bump `AGENTS.md` Ranges to `D1-D27, C1-C5, PIT-1..NN` and add the `status.md` table row — same commit, or C4 will fail.
- [ ] **Step 4:** `bash scripts/gate.sh` → 6 `[PASS]`; `python3 - <<'E'` link check → `0`; C1 grep → empty.
- [ ] **Step 5: commit** `docs(build): plan-10 lane vocabulary, inventory and D27 (T7)`

## Task 8: PolyOrch gitlink (F4)

- [ ] **Step 1:** confirm what is checked out: `git -C PolyOrch rev-parse --short HEAD` (`879856a` after a 2026-10-22 10:45 `pull --tags origin main`, while the recorded pin is `5945f2a`) and that `git -C PolyOrch status --porcelain` is empty.
- [ ] **Step 2:** run Tasks 5 Steps 1-3 **against the checked-out commit**.
- [ ] **Step 3 (F4=P1):** if green → `git add PolyOrch && git commit -m "chore(deps): bump PolyOrch pin 5945f2a..879856a (graph lanes+gates green against it)"` and annotate D25's status.md line. **If red → P2:** `git submodule update PolyOrch`, record the failure in `pitfalls.md`, and re-run the battery to prove the old pin is still the truth.
- [ ] **Step 4:** add a pitfall entry either way: the working tree had moved 3 days of builds without the gitlink being the authority we claim (D25 text), i.e. `git status` carried a ` M PolyOrch` nobody explained. Prevention: `git diff --quiet -- PolyOrch || echo "pin drift"` inside the gate face.

---

## Review Focus (what the plan's tests do not automatically prove)

1. **A silent default goal.** With F1=A, `cmake --build out/cmake-build` builds nothing and exits 0 — a person will believe they built the project. F1=B removes the class; F1=A must at least document it in `docs/modules/09` and the `AGENTS.md` COMMANDS line.
2. **`-L rust` must keep meaning "stub side".** If the live lane were labeled `rust`, `pixi run test` would grow a ~4-minute worker compile and the daily loop would quietly become slow — the exact shape of a lane that stops being used.
3. **Freshness of `embedded-web`.** Merging the handles changes which node the web→server ordering edge hangs on; if the edge is lost, the failure is not a build error but a binary serving stale assets (rust-embed baked at compile time). Task 2 Step 1 owns it; the e2e admin click-through rows are the behavioural net.
4. **`--lane live` on a machine that never built the worker.** Install must fail loudly (missing artifact), never fall back to the stub exe; Task 2 Step 5 checks the positive path — the negative path is a deliberate "does it fail as designed" run.
5. **Manual cargo invocations fork a second cache.** Anything typed without `--target-dir out/cargo-target/<group>` lands in `./target` (measured 33 GB of it), producing wrong freshness conclusions — the rule belongs in `docs/modules/09`, and Task 6 measures whether the two caches should merge at all.
6. **The submodule is a version authority only if the pin is enforced.** `TalkServoPolyOrch.cmake:13-16` promises "never fetches"; the working tree moved anyway (a human `pull`). Task 8 makes the pin match reality or reverts it, and adds a drift check to the gate.

## Blast radius (files that change; nothing outside these)

`cmake/TalkServoCrates.cmake` · `cmake/TalkServoWeb.cmake` · `cmake/TalkServoGates.cmake` (Task 6 only) · `cmake/TalkServoInstall.cmake` · `pixi.toml` · `talkservo.sh` · `docs/modules/09-dev-toolchain.md` · `docs/README.md` · `AGENTS.md` · `.agents/memorys/{decisions,status,pitfalls}.md` · `PolyOrch` gitlink (Task 8) · one-time: cache dirs move → first `pixi run test` pays a cold stub build, first live run pays the worker build again.

**Rollback:** every task is one commit and touches only graph wiring; `git revert` of Task 1-4 restores the old names, and since the cache dirs are gitignored build outputs, `talkservo-lanes-clean` + a rebuild returns the host to a clean state. No data, protocol or code semantics change — the wire contract, features and test counts are untouched.

## Executed-findings log (T4, 2026-10-22) — two facts that only came out by running it

1. **The live lane had never been test-verified, and its command path lacked the linker pin.** The build rule gets
   `CARGO_TARGET_<triple>_LINKER=/usr/bin/cc` from `polyorch_rust_set_env_vars`, but the ctest bridge composed its own
   command through `_polyorch_rust_command` with only `_fenv` — and `cargo test` RE-LINKS the bin and harnesses. With no
   pin the relink takes `cc` off PATH, which inside the pixi env is `.pixi/envs/dev/bin/cc -> x86_64-conda-linux-gnu-cc`,
   whose sysroot lacks `__libc_single_threaded`. Result: `talkservo-server-live-test` failed 3 link units on first run,
   and the pre-existing `pixi run test-sfu` could never have passed. Fix: one `_laneenv` list now feeds the build rule,
   the test bridge and the clean command alike (the W4 property made literal). After it: live lane Passed in 1545s, daily 4/4.
2. **A lane label can silently widen another lane's filter.** Measured, not theorised: `rust-live` joined the daily set;
   `mediasoup` does not. Ratchet kept: `ctest -L "rust|web" -N` must report exactly 4 entries.
