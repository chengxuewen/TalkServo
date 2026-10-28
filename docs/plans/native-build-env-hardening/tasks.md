# Tasks: native-build-env-hardening

Order matters: P1 unblocks everything; P2 makes it durable; P3 documents honestly (C5);
P4 proves the whole chain bare-shell. Each task independently verifiable.

## Phase 1: Lane env injection (P0 root cause #3)

- [x] **Confirm PolyOrch command composition (DONE during 2026-10-27 audit — record, don't redo)**
  - File: `PolyOrch/cmake/PolyOrchFindRust.cmake` `_polyorch_rust_command` (lines 1375-1418):
    strip = {RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS, CFLAGS, CXXFLAGS, CC, CXX}; LD_LIBRARY_PATH
    is NOT stripped; LIBCLANG_PATH is auto-derived when ${POLYORCH_RUST_BIN_DIR}/../lib/libclang.so
    exists (line 1412); PATH prepend uses POLYORCH_RUST_BIN_DIR.
  - Verify: no code change — the notes above ARE the deliverable (audit output)

- [x] **Inject CC/CXX/LD_LIBRARY_PATH into the live lane env**
  - File: `cmake/TalkServoCrates.cmake` (`_laneenv` block, `sfu-mediasoup` branch)
  - Content: `CC=${POLYORCH_RUST_BIN_DIR}/clang`, `CXX=${POLYORCH_RUST_BIN_DIR}/clang++-23`,
    `LD_LIBRARY_PATH=${POLYORCH_RUST_BIN_DIR}/../lib`. **Do NOT reuse `TALKSERVO_CONDA_CC`**:
    `find_program(cc)` resolves to the conda gcc wrapper (`x86_64-conda-linux-gnu-gcc`), not
    the clang pixi.toml activation pins. All values derive from `POLYORCH_RUST_BIN_DIR` — the
    same variable PolyOrch's wrapper already uses for its PATH prepend and LIBCLANG_PATH probe;
    no new scope dependency, no `.pixi/envs/dev` literals. Do NOT inject LIBCLANG_PATH
    (PolyOrch auto-derives it; re-injection races that logic).
  - Verify: `pixi run cmake-probe` → configure output still lists all four lanes; grep the
    generated build rules for `CC=<...>/clang` on the live lane's cargo line
    (prove injection rides the composed env and carries clang, not the gcc wrapper)

- [x] **Prove injection reaches meson**
  - File: (no change — verification task)
  - Verify: bare-shell (non-activated) `cmake --build build --target
    talkservo-server-live-build` after `rm -rf out/cargo-target/live/debug/build/mediasoup-sys-*`;
    then `grep -c "clang++-23" out/cargo-target/live/debug/build/mediasoup-sys-*/out/build/meson-logs/meson-log.txt`
    ≥ 1 (compiler identity proven), and the build completes. (Uses the existing ad-hoc
    `build/` dir — presets land in Phase 3 and the final acceptance re-proves via preset.)

## Phase 2: Worker cache restore (P1 root cause #2)

- [x] **Write scripts/worker-cache-restore.sh**
  - File: `scripts/worker-cache-restore.sh` (new, ~40 lines)
  - Content: locate newest `~/.cargo/registry/src/*/mediasoup-sys-0.14.2/subprojects/`;
    staging dir from `${TALKSERVO_WORKER_CACHE:-$HOME/.cache/talkservo/meson-packagecache}`;
    idempotence short-circuit (existing built `.a` → print + exit 0); per-file sha256 verify
    against the wrap's `source_hash` before copying; staging-empty → print the exact
    missing-file list + curl hints + exit 1 (loud, actionable). `bash -n` clean.
  - Verify: `bash -n scripts/worker-cache-restore.sh && bash scripts/worker-cache-restore.sh`
    → restores 12 files, prints per-file OK

- [x] **Register the pixi task**
  - File: `pixi.toml` (`[feature.dev.tasks]`)
  - Content: `worker-cache-restore = "bash scripts/worker-cache-restore.sh"`
  - Verify: `pixi run worker-cache-restore` runs the script (idempotent second run)

## Phase 3: IDE entry surface (P2)

- [x] **CMakePresets.json**
  - File: `CMakePresets.json` (new)
  - Content: `default` (all custom lanes OFF — acceptance/behavior-neutral), `stub`
    (stub lanes + web, the daily flow), `live` (adds the live lane). buildPresets +
    testPresets inherit. binaryDir `${sourceDir}/build/presets/${presetName}`.
  - Verify: `cmake --list-presets` lists 3; configure `stub` + `live` once each in a scratch
    binaryDir; existing ad-hoc `build/` dir unaffected

- [x] **.vscode/settings.example.json**
  - File: `.vscode/settings.example.json` (new, port of MediaServo's)
  - Content: cmakePath → `${workspaceFolder}/.pixi/envs/dev/bin/cmake`; presets wired.
  - Verify: JSON parses (`python3 -m json.tool`); documented in modules/09 as copy-to-`settings.json`

## Phase 4: Docs, ledger, full verification

- [x] **modules/09 update**
  - File: `docs/modules/09-dev-toolchain.md`
  - Content: §3 — live-lane env injection posture (bare-shell buildable; what's injected and
    why, citing the PIT-14 strip as the mechanism this works WITH, not against); §3 — the
    staging/bootstrap tier for the worker cache (one command, what it needs network for);
    §4 — script inventory row + preset row. Every claim names its command (C5).
  - Verify: `bash scripts/gate.sh` passes (C4/C5/C6 blocks)

- [x] **PIT entry**
  - File: `.agents/memorys/pitfalls.md` (new PIT, next free number)
  - Content: five-section template. Symptom: IDE live build fails with gcc-9.4 `-fconcepts`
    then flatc GLIBCXX. Root cause: activation.env is a human act; PolyOrch strips host
    CC/CXX by design (PIT-14); meson falls back to system gcc; conda-compiled flatc needs
    conda libstdc++ at runtime. Solution: lane-env injection + cache restore + presets.
    Verification: the Phase-1/2 commands.
  - Verify: gate green (C4 index consistency — update AGENTS.md range row in same commit)

- [x] **Full bare-shell acceptance**
  - File: (no change — final verification)
  - Verify: `env -i HOME=$HOME PATH=/usr/bin:/bin:$HOME/.pixi/bin bash -c 'cd <repo> &&
    cmake --build build/presets/live --target talkservo-deploy-build'` → exit 0,
    `out/live/bin/talkservo-server` exists; `pixi run test` + `pixi run gate` green after
    (no regression on the daily flow). Note: `$HOME/.pixi/bin` rides the bare PATH because
    `polyorch_pixi_find` must still locate the pixi binary; the shell stays UNACTIVATED
    (no CC/CXX/LD_LIBRARY_PATH) — the exact condition the lane injection must survive.
    (Audited 2026-10-27: system /usr/bin/cmake 4.4.4 >= the 3.25 floor, so the bare env
    needs no pixi cmake.)

## Completion checklist

- [x] All boxes above checked with real command output (no self-reports — PIT-1)
- [x] `pixi run gate` green; `pixi run test` green
- [x] modules/09 + pitfalls + AGENTS.md ranges updated in the same commit set
- [ ] (future, out of this change) MediaServo port: record pointer in that repo's ledger
      when it happens — this change is the evidence package

## Landing record (2026-10-27, apply session)

- P3-T6 adjudicated =option A during apply: new `TALKSERVO_LIVE_LANE` option (graph-opt-in),
  auto-forced ON when `TALKSERVO_PACKAGE_LANE=live` (declared early in CMakeLists so the
  crates face sees it); presets `stub`/`live` (+hidden `base`) landed accordingly.
- Implementation extension (same root-cause family): the trio moved from a lane-local block
  into `talkservo_mediasoup_toolchain_env()` (TalkServoPolyOrch.cmake, single source);
  `gate-clippy` consumes it too — cold `pixi run gate` had reproduced the gcc-9 concepts
  failure in `target/` (workspace clippy also builds the worker), 900s timeout variant on
  the pre-existing tree. gate-clippy now Passed 164.73s cold.
- Script bug found+fixed during P4-T10: the "worker already built" short-circuit was wrong
  (registry source tree is SHARED across target dirs) — removed; hygiene + per-file sha256
  checks make the script idempotent instead.
- Verification evidence: bare-shell (`env -i`, no activation, PATH=usr+pixi) live deploy
  exit 0 → out/live/bin/talkservo-server (228 MB); meson-log compiler lines show
  clang/clang++-23; `pixi run test` 5/5; `pixi run gate` 4/4 (gate-deny Skipped=absent tool
  path honest); `scripts/gate.sh` 9/9.

## Follow-up landed same day (deploy bin/ completeness, user ruling "no missing dependencies")

- `talkservo-oxmgr-vendor` build-time vendoring replaced the configure-time GLOB (PIT-46);
  deploy-build + --install now guarantee out/<lane>/bin/ = server + oxmgr on fresh clones.
  Verified: fresh wipe -> both binaries staged (0.42s second run), tamper -> refused,
  gate.sh 9/9, pixi run test 5/5.
