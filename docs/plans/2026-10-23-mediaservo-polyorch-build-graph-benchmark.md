# MediaServo / PolyOrch Build Graph Benchmark Adjudication

Date: 2026-10-23  
Status: adjudicated by the user one by one; landed through plan-11 on 2026-10-23.
Scope: CMake/PolyOrch build graph, target vocabulary, test registration, clean semantics, Web lane, IDE entry surface, and lane drift protection.

## Summary Table

| # | Topic | Decision | Key rationale |
|---|---|---|---|
| 1 | PolyOrch supplier baseline | B: adopt PolyOrch `f8f85db` / version `0.7.1` as the new TalkServo build-graph baseline | The current working checkout is already `f8f85db` and supports `polyorch_rust_test(... CTEST ...)`; leaving `HEAD` at `879856a` hides checkout drift. |
| 2 | Rust lane ctest registration | C: normal Rust lanes migrate to supplier `CTEST`; special live gates and web fixture wrappers remain until proven | Supplier CTEST removes duplicated bridge code, but TalkServo still needs explicit runtime gating and `web-dist-fixture` ordering. |
| 3 | PolyOrch embed shim | A: harden `TalkServoPolyOrch.cmake` with sentinel property floor, `EXCLUDE_FROM_ALL`, and repo anchor | Version floor makes baseline B real; excluding vendor examples/tests prevents supplier pollution; repo-anchor keeps superbuild behavior sane. |
| 4 | Clean posture | A: keep cache-group clean; do not add per-package clean yet | Current stub/live isolation and measured `CACHEDIR.TAG` refusal make group clean the stable choice for PoC. |
| 5 | Target naming | A: keep `talkservo-<object>-<verb>` | The prefix encodes graph ownership, lane config, and action; D27 already uses this vocabulary across docs, scripts, labels, and install knobs. |
| 6 | Web lane modeling | C: keep `talkservo-web-*`; defer `polyorch_node_import` until a real multi-member frontend workspace exists | Current graph has only a small Web lane; importing a full node workspace pattern would be premature. |
| 7 | IDE / configure presets | B: do not introduce CMakePresets or a MediaServo-style option matrix now | TalkServo entry authority remains docs / pixi / `talkservo.sh`; presets would create a second entry path before there is demand. |
| 8 | Lane drift protection | A: add a TalkServo lane reconciliation gate with self-test | D27 and plan-10 showed that target names, ctest labels, docs inventory, install handles, and Cargo packages can drift; C4 culture says indexes should be machine-checked. |

## Detailed Decisions

### 1. PolyOrch supplier baseline

TalkServo should treat PolyOrch `f8f85db88240590e20dd910414a7ef3fd6d81982`, version `0.7.1`, as the intended supplier baseline.

Current state found during the benchmark:

```text
TalkServo HEAD gitlink:
879856a85f9d046b153e7df415e87af3b73b6a2a

TalkServo PolyOrch checkout:
f8f85db88240590e20dd910414a7ef3fd6d81982
branch: dev-mediaservo
version: 0.7.1
```

Implementation implication:

- A controlled submodule bump is needed.
- The current checkout drift must be explained and removed.
- Fresh-clone verification must test the new pin.

### 2. Rust lane ctest registration

Normal Rust lanes should move toward:

```cmake
polyorch_rust_test(
  PACKAGE <cargo-package>
  NAME <talkservo-handle-test>
  CTEST
  CTEST_NAME <talkservo-handle-test>
  CTEST_LABELS <labels>
  CTEST_WORKING_DIRECTORY <repo-root>
)
```

Special lanes can remain hand-wrapped when they need behavior that supplier CTEST does not yet clearly provide:

- `talkservo-server-live-test` runtime gating through `TS_LANE_LIVE=1`.
- `web-dist-fixture` ordering for embedded-web lanes.
- Gate/e2e entries that are not ordinary Rust package tests.

### 3. PolyOrch embed shim hardening

`cmake/TalkServoPolyOrch.cmake` should move from a `COMMAND`-only check to a versioned sentinel check.

Target shape:

```cmake
if(NOT TARGET PolyOrch::PolyOrch)
  message(FATAL_ERROR
    "PolyOrch was added but did not install PolyOrch::PolyOrch. Expected a CMake 3.25+ PolyOrch tree.")
endif()

get_property(_po_dir TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_CMAKE_DIR)
get_property(_po_ver TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_VERSION)

if(NOT _po_dir)
  message(FATAL_ERROR
    "PolyOrch::PolyOrch exists without INTERFACE_POLYORCH_CMAKE_DIR. This checkout is too old or bare.")
endif()

if(NOT _po_ver OR _po_ver VERSION_LESS 0.7.1)
  message(FATAL_ERROR
    "TalkServo requires PolyOrch >= 0.7.1 (found '${_po_ver}'). Run git submodule update --init --recursive.")
endif()
```

The self-embed should also use `EXCLUDE_FROM_ALL` so PolyOrch's own examples/tests do not become TalkServo build targets.

### 4. Clean posture

Keep the current cache-group deletion model:

```text
talkservo-lanes-stub-clean
talkservo-lanes-live-clean
talkservo-lanes-clean
talkservo-web-clean
```

Do not add these yet:

```text
talkservo-core-clean
talkservo-sfu-clean
talkservo-server-stub-clean
talkservo-server-live-clean
```

Revisit per-package clean only when TalkServo introduces a shared cargo target posture, FFI/delivery needs, or MediaServo-style `host/server/all` presets.

### 5. Target naming

Keep D27:

```text
talkservo-<object>-<verb>
verb = build | test | clean
```

Examples:

```text
talkservo-core-build
talkservo-sfu-test
talkservo-server-stub-build
talkservo-server-live-test
talkservo-lanes-stub-build
talkservo-web-clean
```

Do not revert to bare crate names such as `core-test` or `server-build`.

### 6. Web lane

Keep the current web trio:

```text
talkservo-web-build
talkservo-web-test
talkservo-web-clean
```

Trigger for future migration to `polyorch_node_import(...)`:

- `web/` or a future `www/` becomes a real multi-member workspace.
- `packages/client`, admin, or desktop start sharing node build graph edges.
- The hand-written `web-test` ctest bridge begins to drift repeatedly from button/ctest expectations.

### 7. IDE / configure preset posture

No new `CMakePresets.json` for now.

The current entry authority remains:

```text
docs/modules/09
pixi tasks
talkservo.sh forwarders
```

A future minimal preset could be considered only when there is a concrete VS Code / CMake Tools workflow to support. A full MediaServo-style preset matrix is premature.

### 8. Lane reconciliation gate

Add a small TalkServo gate that checks at least these fact sources against each other:

```text
Cargo workspace members
TalkServoCrates.cmake lane registrations
talkservo-* target inventory
docs/modules/09 lane/target table
TALKSERVO_PACKAGE_LANE values
ctest name/label expectations
```

Recommended shape:

```text
scripts/gate-lanes.py
scripts/gate-lanes.py --self-test
```

The self-test should include a fixture where a planted missing lane fails. The gate should be wired into `scripts/gate.sh` as a ledger/docs/build-graph consistency check.

## Open Items

- Implement the controlled PolyOrch gitlink bump and remove the current checkout/index drift.
- Verify fresh-clone behavior against PolyOrch `0.7.1`.
- Validate supplier `CTEST` behavior with pixi env, linker pins, labels, and working directory.
- Decide which exact fields the lane reconciliation gate will parse first: CMake text, Cargo.toml text, docs table, or configured ctest output.
- Record the implementation decision in a D-entry or module update if this plan is converted into an actual build-graph change.