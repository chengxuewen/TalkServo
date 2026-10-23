# PolyOrch Build Graph Revision Implementation Plan

**Status:** implemented and verified locally (2026-10-23)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the eight adjudicated MediaServo/PolyOrch benchmark decisions in TalkServo's CMake build graph without adopting per-package clean, CMakePresets, Web workspace import, or SDK/bindings install scope.

**Architecture:** Cargo and npm remain the build authorities; PolyOrch generates the CMake buttons and ctest projections; TalkServo CMake remains a thin assembly surface; local scripts and `scripts/gate.sh` provide drift protection. The revision first records the PolyOrch `0.7.1` baseline, then hardens the embed shim, migrates normal Rust lanes to supplier `CTEST`, and adds a static lane reconciliation gate.

**Tech Stack:** CMake `>= 3.25`, PolyOrch `0.7.1` (`f8f85db88240590e20dd910414a7ef3fd6d81982`), Cargo/pixi, CTest, Bash, Python 3 stdlib, npm/Vite web lane, mediasoup live lane behind `TS_LANE_LIVE=1`.

**Spec:** `docs/plans/2026-10-23-mediaservo-polyorch-build-graph-benchmark.md`

## Global Constraints

- PolyOrch supplier baseline is exactly `f8f85db88240590e20dd910414a7ef3fd6d81982` and version `0.7.1`.
- PolyOrch minimum version floor is `0.7.1`; a bare or older checkout must fail at configure time.
- Target naming remains `talkservo-<object>-<verb>` with verbs `build`, `test`, and `clean`.
- Clean posture remains cache-group only for Rust lanes: `talkservo-lanes-stub-clean`, `talkservo-lanes-live-clean`, `talkservo-lanes-clean`; Web keeps `talkservo-web-clean`.
- Do not add per-package clean targets such as `talkservo-core-clean`, `talkservo-sfu-clean`, `talkservo-server-stub-clean`, or `talkservo-server-live-clean`.
- Do not add `CMakePresets.json`, `.vscode` presets, or a MediaServo-style preset matrix in this plan.
- Do not call hosted or automatic verification as if it runs; local commands are the enforcement path. Name commands or use `inert` / `D26`.
- Do not start SDK, FFI, bindings, or desktop install staging from this build-graph revision.
- Live lane tests remain runtime gated; `talkservo-server-live-test` must not enter the daily `rust|web` set unless `TS_LANE_LIVE=1`.
- Do not stage `.omo/run-continuation/` files.
- Do not push unless the user explicitly asks to push.

## Review Focus

- Old or bare PolyOrch checkout silently accepted: must be rejected by the embed shim before lane registration.
- Supplier `CTEST` migration loses pixi environment, linker pins, labels, working directory, or generated binary-name semantics: verify by reconfiguring and running daily lanes.
- Supplier `CTEST` migration omits per-lane `ARGS` feature flags: verify generated `CTestTestfile.cmake` still contains `--no-default-features --features stub-media` for the SFU stub lane and `--no-default-features --features stub-media,embedded-web` for the server stub lane.
- Live lane runtime gate is accidentally removed or widened: verify `talkservo-server-live-test` still requires `TS_LANE_LIVE=1`.
- Fresh clone after submodule bump cannot reproduce the graph: verify submodule status, clean configure, and daily tests.
- Lane reconciliation gate reports green from empty parses: self-test must prove a planted missing lane fails.

---

### Task 1: Record the PolyOrch 0.7.1 Supplier Baseline

**Files:**
- Modify: `PolyOrch` gitlink
- Modify: `docs/modules/09-dev-toolchain.md`
- Modify: `docs/plans/2026-10-23-mediaservo-polyorch-build-graph-benchmark.md` only if the final text needs the recorded SHA after landing

**Interfaces:**
- Consumes: current working checkout `PolyOrch` at `f8f85db88240590e20dd910414a7ef3fd6d81982`.
- Produces: repository index records the same PolyOrch commit as the intended supplier baseline.

- [ ] **Step 1: Capture the drift as a failing check**

Run:

```bash
! git submodule status PolyOrch | head -n 1 | grep -q '^[+-]'
```

Expected: exit code `1` while the leading `+` still marks the submodule checkout as different from the recorded gitlink.

- [ ] **Step 2: Stage only the PolyOrch gitlink**

Run:

```bash
git add PolyOrch
```

Expected: no unrelated paths are staged.

- [ ] **Step 3: Commit the supplier baseline**

Run:

```bash
git commit -m "build(submodule): record PolyOrch 0.7.1 supplier baseline"
```

Expected: commit succeeds.

- [ ] **Step 4: Re-run the drift check**

Run:

```bash
! git submodule status PolyOrch | head -n 1 | grep -q '^[+-]'
```

Expected: exit code `0` when the status line begins with a space, meaning the checked-out submodule matches the recorded gitlink.

- [ ] **Step 5: Update the toolchain doc with the recorded supplier baseline**

Modify `docs/modules/09-dev-toolchain.md` so the PolyOrch section states:

```text
PolyOrch supplier baseline:
f8f85db88240590e20dd910414a7ef3fd6d81982
PolyOrch version: 0.7.1
```

Keep English only; no Chinese text.

- [ ] **Step 6: Verify the existing graph still configures**

Run:

```bash
~/.pixi/bin/pixi run cmake-probe
```

Expected: configure succeeds. Do not proceed if configure fails.

- [ ] **Step 7: Commit docs baseline**

Run:

```bash
git add docs/modules/09-dev-toolchain.md
git commit -m "docs(build): record PolyOrch 0.7.1 supplier baseline"
```

---

### Task 2: Harden the PolyOrch Embed Shim

**Files:**
- Create: `scripts/gate-polyorch.sh`
- Modify: `scripts/gate.sh`
- Modify: `cmake/TalkServoPolyOrch.cmake`
- Modify: `docs/modules/09-dev-toolchain.md`

**Interfaces:**
- Consumes: `PolyOrch::PolyOrch` sentinel properties `INTERFACE_POLYORCH_CMAKE_DIR` and `INTERFACE_POLYORCH_VERSION`.
- Produces: `bash scripts/gate-polyorch.sh` returns `0` only when the current embed path has a sentinel and rejects bare or old PolyOrch trees.

- [ ] **Step 1: Write the failing PolyOrch floor gate**

Create `scripts/gate-polyorch.sh` with this contract:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/bare-polyorch" "$TMP/old-polyorch" "$TMP/test-project"

cat > "$TMP/bare-polyorch/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.25)
project(PolyOrch VERSION 0.3.0 LANGUAGES NONE)
EOF

cat > "$TMP/old-polyorch/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.25)
project(PolyOrch VERSION 0.3.0 LANGUAGES NONE)
add_library(PolyOrch::PolyOrch INTERFACE IMPORTED GLOBAL)
set_property(TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_CMAKE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
set_property(TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_VERSION "${PROJECT_VERSION}")
EOF

cat > "$TMP/test-project/CMakeLists.txt" <<EOF
cmake_minimum_required(VERSION 3.25)
project(TalkServoPolyOrchGateTest LANGUAGES CXX)
include("$ROOT/cmake/TalkServoPolyOrch.cmake")
talkservo_ensure_polyorch()
EOF

for label in bare old; do
  dir="$TMP/${label}-polyorch"
  build="$TMP/build-${label}"
  log="$TMP/log-${label}"
  if cmake -S "$TMP/test-project" -B "$build" -DTALKSERVO_POLYORCH_DIR="$dir" > "$log" 2>&1; then
    echo "FAIL: PolyOrch ${label} fixture was accepted"
    exit 1
  fi
done

grep -qi 'PolyOrch was added but did not install PolyOrch::PolyOrch' "$TMP/log-bare"
grep -qi 'TalkServo requires PolyOrch >= 0.7.1' "$TMP/log-old"

echo PASS
```

- [ ] **Step 2: Run the gate to prove it fails before the shim is hardened**

Run:

```bash
bash scripts/gate-polyorch.sh
```

Expected: FAIL because the current shim only checks `COMMAND polyorch_rust_setup` and does not enforce the sentinel/version floor.

- [ ] **Step 3: Implement sentinel property checks in the shim**

Modify `cmake/TalkServoPolyOrch.cmake`:

- Add a local helper `talkservo_require_polyorch_sentinel()` in `cmake/TalkServoPolyOrch.cmake`:

```cmake
function(talkservo_require_polyorch_sentinel)
  if(NOT TARGET PolyOrch::PolyOrch)
    message(FATAL_ERROR
      "PolyOrch was added but did not install PolyOrch::PolyOrch. TalkServo requires PolyOrch 0.7.1.")
  endif()

  get_property(_po_dir TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_CMAKE_DIR)
  get_property(_po_ver TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_VERSION)

  if(NOT _po_dir)
    message(FATAL_ERROR
      "PolyOrch was added but did not install PolyOrch::PolyOrch with INTERFACE_POLYORCH_CMAKE_DIR.")
  endif()

  if(NOT _po_ver OR _po_ver VERSION_LESS 0.7.1)
    message(FATAL_ERROR
      "TalkServo requires PolyOrch >= 0.7.1 (found '${_po_ver}'). Run git submodule update --init --recursive.")
  endif()
endfunction()
```

- Reuse the outer scope only after `talkservo_require_polyorch_sentinel()` passes. Do not reuse a bare `polyorch_rust_setup` command without the sentinel target.
- Emit the fatal phrase `PolyOrch was added but did not install PolyOrch::PolyOrch` when the sentinel target or its directory property is missing.
- Emit the fatal phrase `TalkServo requires PolyOrch >= 0.7.1` when the version property is missing or too old.

- [ ] **Step 4: Add `EXCLUDE_FROM_ALL` to the self-embed**

Modify the current `add_subdirectory("${_po}" "${CMAKE_BINARY_DIR}/_polyorch" SYSTEM)` call so vendor example/test targets do not enter the host `ALL` set:

```cmake
add_subdirectory("${_po}" "${CMAKE_BINARY_DIR}/_polyorch" SYSTEM EXCLUDE_FROM_ALL)
```

- [ ] **Step 5: Run the gate to verify it passes**

Run:

```bash
bash scripts/gate-polyorch.sh
```

Expected: stdout ends with `PASS` and exit code `0`.

- [ ] **Step 6: Wire the gate into `scripts/gate.sh`**

Add a manual block after C5 and before LINK:

```bash
echo "=== POLYORCH ==="
if bash scripts/gate-polyorch.sh; then echo "[PASS] POLYORCH"; else echo "[FAIL] POLYORCH"; rc=1; fi
```

Do not put it inside the managed C1-C5 base64 blocks.

- [ ] **Step 7: Update docs/modules/09 for the embed contract**

Add or replace the PolyOrch embed note with these facts:

```text
TalkServo embeds PolyOrch through the PolyOrch::PolyOrch sentinel.
TalkServo requires INTERFACE_POLYORCH_VERSION >= 0.7.1.
The self-embed uses EXCLUDE_FROM_ALL so PolyOrch's own example/test targets do not become TalkServo build targets.
```

- [ ] **Step 8: Run ledger gates**

Run:

```bash
bash scripts/gate.sh
```

Expected: all blocks pass, including the new `POLYORCH` block.

- [ ] **Step 9: Commit**

Run:

```bash
git add cmake/TalkServoPolyOrch.cmake scripts/gate-polyorch.sh scripts/gate.sh docs/modules/09-dev-toolchain.md
git commit -m "fix(build): enforce PolyOrch 0.7.1 sentinel floor and exclude vendor targets"
```

---

### Task 3: Migrate Normal Rust Lanes to Supplier `CTEST`

**Files:**
- Modify: `cmake/TalkServoCrates.cmake`
- Modify: `docs/modules/09-dev-toolchain.md`

**Interfaces:**
- Consumes: `polyorch_rust_test(PACKAGE ... NAME ... TARGET ... MANIFEST ... BASE_DIR ... FOLDER ... ARGS ... CTEST CTEST_NAME ... CTEST_LABELS ... CTEST_WORKING_DIRECTORY ... CTEST_SKIP_RETURN_CODE ...)` from PolyOrch `0.7.1`.
- Produces: ungated Rust lanes have the same generated button and CTest entry, named `talkservo-core-test`, `talkservo-sfu-test`, and `talkservo-server-stub-test`, and their ctest command vectors still carry the lane feature flags.

- [ ] **Step 1: Capture the current daily test baseline**

Run:

```bash
~/.pixi/bin/pixi run test
```

Expected: command exits `0`. Record pass/fail output. Do not proceed if there is a failing baseline unrelated to this task.

- [ ] **Step 2: Rewrite `talkservo_rust_lane()` CTest registration**

In `cmake/TalkServoCrates.cmake`, change the `if(L_TEST)` path:

- Build the feature argument list before registering the test verb:

```cmake
if(L_FEATURES)
  set(_test_args --no-default-features --features "${L_FEATURES}")
else()
  set(_test_args "")
endif()
```

- For ungated Rust lanes, call PolyOrch once with `CTEST` and pass `ARGS ${_test_args}`:

```cmake
polyorch_rust_test(
  PACKAGE  ${L_PACKAGE}
  NAME     ${L_HANDLE}-test
  TARGET   ${L_HANDLE}
  MANIFEST "${_manifest}"
  BASE_DIR "${_base}"
  FOLDER   "${_folder}"
  ARGS     ${_test_args}
  CTEST
  CTEST_NAME ${L_HANDLE}-test
  CTEST_LABELS "${L_TEST_LABELS}"
  CTEST_WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
  CTEST_SKIP_RETURN_CODE 77
)
```

- For lanes with `TEST_GATE`, keep `polyorch_rust_test` without `CTEST`, keep the current `_polyorch_rust_forward_env` + `_polyorch_rust_command` + `lane-gate.sh` path, and keep `SKIP_RETURN_CODE 77`. The live lane must remain runtime-gated through `scripts/lane-gate.sh` and `_laneenv`.

- Do not keep duplicate `add_test(...)` entries for ungated lanes.
- Do not rely on `set_tests_properties` after the hand bridge for ungated lanes; PolyOrch's `CTEST` path owns those properties.

- [ ] **Step 3: Update the outdated bridge comment**

Replace the current comment claiming `polyorch_rust_test` only registers a BUILD target with text like:

```text
PolyOrch 0.7.1 can register the button and ctest entry from one CTEST call.
Ungated Rust lanes use that path. Runtime-gated live lanes keep a wrapper so
TS_LANE_LIVE can still decide at run time.
```

- [ ] **Step 4: Reconfigure the graph**

Run:

```bash
rm -rf out/cmake-build-polyorch-ctest
~/.pixi/bin/pixi run cmake-probe
cmake -S . -B out/cmake-build-polyorch-ctest -G Ninja
```

Expected: both configure passes succeed; `out/cmake-build` is refreshed so later `pixi run test` verifies the migrated graph, and `out/cmake-build-polyorch-ctest` is a clean migration-only configuration.

- [ ] **Step 5: Verify CTest names and labels**

Run:

```bash
ctest --test-dir out/cmake-build-polyorch-ctest -N -R '^talkservo-(core|sfu|server-stub)-test$'
ctest --test-dir out/cmake-build-polyorch-ctest -N -R '^talkservo-server-live-test$'
ctest --test-dir out/cmake-build-polyorch-ctest -N -L mediasoup
```

Expected:
- three entries for core/sfu/server-stub,
- one live entry,
- the live entry carries `mediasoup` label and does not appear as a normal `rust` label.

Run:

```bash
grep -q 'talkservo-server-stub-test.*stub-media,embedded-web' out/cmake-build-polyorch-ctest/CTestTestfile.cmake
grep -q 'talkservo-sfu-test.*stub-media' out/cmake-build-polyorch-ctest/CTestTestfile.cmake
! grep -E -q 'talkservo-(core|sfu|server-stub)-test.*sfu-mediasoup' out/cmake-build-polyorch-ctest/CTestTestfile.cmake
```

Expected: the three checks exit `0`. These greps prove the migrated CTEST entries did not silently fall back to the Cargo default `sfu-mediasoup` feature.

- [ ] **Step 6: Run the existing stub graph test task**

Run:

```bash
~/.pixi/bin/pixi run test
```

Expected: command exits `0` against the refreshed `out/cmake-build` configuration.

- [ ] **Step 7: Verify live lane still skips by default**

Run:

```bash
ctest --test-dir out/cmake-build-polyorch-ctest -L mediasoup
```

Expected: live entry is Skipped when `TS_LANE_LIVE` is unset. Do not run live mediasoup tests unless the user explicitly approves the expensive command.

- [ ] **Step 8: Update docs/modules/09**

State these facts:

```text
Normal Rust lanes use polyorch_rust_test(... CTEST ...) on PolyOrch 0.7.1.
The live server lane remains wrapper-gated because TS_LANE_LIVE is a run-time condition.
The ctest label for the live lane remains mediasoup;headless;slow.
```

- [ ] **Step 9: Run ledger gates**

Run:

```bash
bash scripts/gate.sh
```

Expected: all blocks pass.

- [ ] **Step 10: Commit**

Run:

```bash
git add cmake/TalkServoCrates.cmake docs/modules/09-dev-toolchain.md
git commit -m "fix(build): use PolyOrch CTEST for ungated Rust lanes"
```

---

### Task 4: Add Lane Reconciliation Gate

**Files:**
- Create: `scripts/gate-lanes.py`
- Modify: `scripts/gate.sh`
- Modify: `docs/modules/09-dev-toolchain.md`
- Modify: `.agents/rules/common/testing.md` only if a new ratchet is needed

**Interfaces:**
- Consumes: `Cargo.toml`, `cmake/TalkServoCrates.cmake`, `cmake/TalkServoInstall.cmake`, `cmake/TalkServoGates.cmake`, `cmake/TalkServoWeb.cmake`, `docs/modules/09-dev-toolchain.md`.
- Produces: `python3 scripts/gate-lanes.py` exits `0` when the lane inventory, docs table, clean targets, labels, install package lane, and Cargo members agree.

- [ ] **Step 1: Write the failing self-test first**

Create `scripts/gate-lanes.py` with:

```python
def self_test() -> int:
    raise NotImplementedError
```

Run:

```bash
python3 scripts/gate-lanes.py --self-test
```

Expected: FAIL because self-test is not implemented.

- [ ] **Step 2: Implement self-test behavior**

Replace the placeholder so `--self-test`:

- builds a valid in-memory lane table,
- returns success,
- returns failure when one required row is removed,
- returns failure when `talkservo-server-live-test` is given the substring label `rust-live`.

Use only Python stdlib.

- [ ] **Step 3: Define required fact sources**

Implement parsing helpers for these exact checks:

1. Cargo workspace members contain exactly:

```text
talkservo-core
talkservo-sfu
talkservo-server
```

2. `TalkServoCrates.cmake` lane registrations contain exactly these handle/package/label/cache rows:

```text
talkservo-core / talkservo-core / rust;headless / stub
talkservo-sfu / talkservo-sfu / rust;headless / stub
talkservo-server-stub / talkservo-server / rust;headless / stub
talkservo-server-live / talkservo-server / mediasoup;headless;slow / live
```

3. `docs/modules/09-dev-toolchain.md` lane table has one row for each handle name:

```text
talkservo-core
talkservo-sfu
talkservo-server-stub
talkservo-server-live
talkservo-web
```

4. Allowed clean target names are exactly:

```text
talkservo-lanes-stub-clean
talkservo-lanes-live-clean
talkservo-lanes-clean
talkservo-web-clean
```

5. `cmake/TalkServoInstall.cmake` composes install handle as:

```cmake
talkservo-server-${TALKSERVO_PACKAGE_LANE}
```

6. No file named by the gate contains a clean target name for an ordinary Rust lane:

```text
talkservo-core-clean
talkservo-sfu-clean
talkservo-server-stub-clean
talkservo-server-live-clean
```

7. No Rust lane uses a label containing the substring `rust-live`.

- [ ] **Step 4: Add failure messages that name the drifting fact source**

Each failure path must print a line containing:
- the fact source path,
- the missing or unexpected value,
- the expected value.

Example:

```text
FAIL: docs/modules/09-dev-toolchain.md missing lane row for talkservo-server-live
```

- [ ] **Step 5: Run the self-test**

Run:

```bash
python3 scripts/gate-lanes.py --self-test
```

Expected: `PASS`.

- [ ] **Step 6: Run the real gate**

Run:

```bash
python3 scripts/gate-lanes.py
```

Expected: `PASS` if the repo inventory is consistent. If it fails, fix the doc/table/source mismatch; do not weaken the gate to match drift.

- [ ] **Step 7: Wire the lane gate into `scripts/gate.sh`**

Add a manual block after the `POLYORCH` block and before `LINK`:

```bash
echo "=== LANES ==="
if python3 scripts/gate-lanes.py; then echo "[PASS] LANES"; else echo "[FAIL] LANES"; rc=1; fi
```

- [ ] **Step 8: Add a testing ratchet**

Update `.agents/rules/common/testing.md` with a short rule:

```bash
python3 scripts/gate-lanes.py --self-test
```

The rule must say: the self-test is required because a static parser can report green on an empty parse.

- [ ] **Step 9: Update docs/modules/09 to reference the gate**

Add a line:

```text
`scripts/gate-lanes.py --self-test` checks that Cargo members, CMake lane registrations, docs inventory, install handles, clean targets, and ctest labels remain one registry.
```

- [ ] **Step 10: Run full local gates**

Run:

```bash
bash scripts/gate.sh
```

Expected: all blocks pass.

- [ ] **Step 11: Commit**

Run:

```bash
git add scripts/gate-lanes.py scripts/gate.sh docs/modules/09-dev-toolchain.md .agents/rules/common/testing.md
git commit -m "feat(gates): reconcile build lane inventory with docs and Cargo"
```

---

### Task 5: Final Revision Audit and Fresh-Clone Proof

**Files:**
- Modify: `docs/README.md`
- Modify: `.agents/memorys/status.md`
- Modify: `AGENTS.md` only if decision ranges change
- Modify: `docs/plans/2026-10-23-mediaservo-polyorch-build-graph-benchmark.md`
- Modify: `docs/plans/2026-10-23-plan11-polyorch-build-graph-revision.md`

**Interfaces:**
- Consumes: all prior tasks.
- Produces: a reviewable landed state, or a blocked report with exact failed command output.

- [ ] **Step 1: Confirm no forbidden surface was added**

Run:

```bash
test ! -e CMakePresets.json
test ! -e .vscode
grep -R -E 'talkservo-(core|sfu|server-stub|server-live)-clean' cmake docs scripts pixi.toml && exit 1 || true
```

Expected: no forbidden preset/VS Code file and no lane-specific Rust clean target.

- [ ] **Step 2: Run configure and daily tests from a clean graph directory**

Run:

```bash
rm -rf out/cmake-build-final
cmake -S . -B out/cmake-build-final -G Ninja
cmake --build out/cmake-build-final --target talkservo-lanes-stub-build
ctest --test-dir out/cmake-build-final -L "rust|web" --output-on-failure
```

Expected: configure, build, and selected tests pass. If web provisioning is missing, run `pixi run client-install` and `pixi run web-install` first, then repeat.

- [ ] **Step 3: Run local ledger gates**

Run:

```bash
bash scripts/gate.sh
```

Expected: all blocks pass, including `POLYORCH` and `LANES`.

- [ ] **Step 4: Verify submodule baseline**

Run:

```bash
git submodule status PolyOrch
git ls-tree HEAD PolyOrch
```

Expected:
- `git submodule status PolyOrch` has no leading `+` or `-`,
- recorded gitlink is `f8f85db88240590e20dd910414a7ef3fd6d81982`.

- [ ] **Step 5: Add the plan row to `docs/README.md`**

Append to the Implementation plans table:

```markdown
| `plans/2026-10-23-plan11-polyorch-build-graph-revision.md` | PolyOrch 0.7.1 baseline, CTEST migration, embed floor, lane reconciliation | PENDING EXECUTION |
```

When the implementation lands, update the status cell to `EXECUTED YYYY-MM-DD`.

- [ ] **Step 6: Update `.agents/memorys/status.md` only after code lands**

Add a short status note:

```text
plan-11 build-graph revision: PolyOrch 0.7.1 supplier floor, ungated Rust lanes via supplier CTEST, live/web special cases still wrapper-gated, lane reconciliation gate landed.
```

Do not create a new decision id unless the user explicitly approves a D-entry.

- [ ] **Step 7: Run C4 gate after status updates**

Run:

```bash
bash scripts/gate.sh
```

Expected: `C4` passes.

- [ ] **Step 8: Mark plan status in the plan file**

Change the top `Status:` line from `PENDING REVIEW` to either:

```text
Status: implemented and verified locally
```

or, if blocked:

```text
Status: blocked by <exact failed command>
```

- [ ] **Step 9: Commit final status/docs only if earlier tasks were already approved for commits**

Run:

```bash
git add docs/README.md docs/plans/2026-10-23-mediaservo-polyorch-build-graph-revision.md docs/plans/2026-10-23-mediaservo-polyorch-build-graph-benchmark.md
git commit -m "docs(build): mark PolyOrch build graph revision status"
```

Do not push.

---

## Explicit No-Implementation List

These items are intentional from the adjudication and must not be added by this plan:

- No `CMakePresets.json`.
- No `.vscode` preset settings.
- No per-package Rust clean targets.
- No shared `target/` posture.
- No `polyorch_node_import(...)` migration.
- No FFI/bindings install face.
- No SDK publish or desktop shell work.
- No claim that hosted CI runs any of these gates.

## Verification Matrix

| Change | Minimum proof |
|---|---|
| PolyOrch gitlink recorded | `git submodule status PolyOrch` has no leading `+`; `git ls-tree HEAD PolyOrch` shows `f8f85db...` |
| Embed shim floor | `bash scripts/gate-polyorch.sh` passes; bare and old fixtures fail configure |
| Rust lane `CTEST` migration | `pixi run test` passes; `ctest -N -R '^talkservo-(core|sfu|server-stub)-test$'` shows 3 entries |
| Live lane remains gated | `ctest --test-dir out/cmake-build -L mediasoup` skips without `TS_LANE_LIVE=1` |
| Lane reconciliation gate | `python3 scripts/gate-lanes.py --self-test` passes; planted missing lane fails |
| Docs consistency | `bash scripts/gate.sh` passes; docs/modules/09 says supplier floor, clean posture, no presets |
| Fresh-clone readiness | clean configure + stub aggregate build + daily `ctest -L "rust|web"` pass in a fresh graph directory |