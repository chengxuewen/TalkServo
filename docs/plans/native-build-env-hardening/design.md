# Design: native-build-env-hardening

**Depends on**: [proposal.md](proposal.md) — read that first for the why and the failure chain.

## Architecture

No runtime architecture changes. The change lives in the build-graph recipe layer:

```
                        IDE button / CLI (any shell, even bare)
                                        │
                        cmake --build build --target talkservo-deploy-build
                                        │
                     ┌──────────────────┴───────────────────┐
                     │  talkservo-server-live-build lane    │
                     │  (TalkServoCrates.cmake)             │
                     │                                      │
                     │  cmake -E env <composed prefix>      │
                     │    cargo ...                         │
                     │      │                               │
                     │  composed prefix order:              │
                     │    1. host-leak strip (PIT-14)       │
                     │       --unset CC/CXX/CFLAGS/...      │
                     │    2. lane env (set_env_vars)  ◄─────┼── THIS CHANGE:
                     │       LINKER, NO_PROXY  (existing)   │   + CC, CXX,
                     │       + CC, CXX, LD_LIBRARY_PATH     │     LD_LIBRARY_PATH
                     │    3. PATH = pixi bin dir (existing) │
                     └──────────────────────────────────────┘
                                        │
                              mediasoup-sys build.rs
                                        │
                     ┌──────────────────┴───────────────────┐
                     │ python invoke → meson                │
                     │   subprojects: 6 C++ deps            │
                     │   ◄── packagecache (restorable ◄─────┼── THIS CHANGE:
                     │       from staging via script)       │   worker-cache-restore
                     │   flatc (conda clang, needs          │     + staging dir
                     │     LD_LIBRARY_PATH=conda lib) ◄─────┘
                     └──────────────────────────────────────┘
```

The two changes attack disjoint layers of the same failure: env injection fixes the
*toolchain identity* layer (which compiler, which libstdc++ at runtime); cache restore
fixes the *source availability* layer (which tarballs meson can fetch offline).

## Files to Touch

### Create

| File | Purpose |
|---|---|
| `scripts/worker-cache-restore.sh` | Copy sha256-verified tarballs from staging (`${TALKSERVO_WORKER_CACHE:-$HOME/.cache/talkservo/meson-packagecache}`) into the active mediasoup-sys `subprojects/packagecache/`. ~40 lines bash. No cargo/pixi dependency. |
| `CMakePresets.json` | Three configure presets: `default` (all lanes off — behavior-neutral acceptance, mirrors MediaServo), `stub` (stub lanes + web), `live` (adds live lane). Build/test presets inherit. binaryDir = `${sourceDir}/build/presets/${presetName}` — keeps the existing ad-hoc `build/` dir untouched. |
| `.vscode/settings.example.json` | Port of MediaServo's: `cmake.cmakePath` pinned to `.pixi/envs/dev/bin/cmake`, presets wired, configure-on-open. Example-suffix per MediaServo convention (no forced settings on clone). |

### Modify

| File | Change |
|---|---|
| `cmake/TalkServoCrates.cmake` | In `talkservo_rust_lane`, extend the `_laneenv` block (the `L_TEST_GATE OR L_FEATURES MATCHES "sfu-mediasoup"` branch, lines ~104-119): append three entries — `CC=${POLYORCH_RUST_BIN_DIR}/clang`, `CXX=${POLYORCH_RUST_BIN_DIR}/clang++-23`, `LD_LIBRARY_PATH=${POLYORCH_RUST_BIN_DIR}/../lib`. **Reviewer correction (2026-10-27 audit): do NOT reuse `TALKSERVO_CONDA_CC` — `find_program(cc)` resolves to `.pixi/envs/dev/bin/cc -> x86_64-conda-linux-gnu-gcc`, a gcc wrapper, NOT the clang that pixi.toml activation.env pins.** The toolchain-relative values derive from `POLYORCH_RUST_BIN_DIR` — the same variable PolyOrch's own command wrapper already uses for its PATH prepend and its LIBCLANG_PATH probe (PolyOrchFindRust.cmake:1404-1418), so no new scope dependency is introduced. Strip interaction, read off `_polyorch_rust_command` (PolyOrchFindRust.cmake:1390-1397): the strip unsets `RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS/CFLAGS/CXXFLAGS/CC/CXX` — CC/CXX injected via `R_ENV` trail the strip and win; **`LD_LIBRARY_PATH` is NOT in the strip list**, so it also survives (the original design note wrongly claimed it needed the after-strip rule). Do NOT inject `LIBCLANG_PATH`: PolyOrch already derives it automatically when `${BIN_DIR}/../lib/libclang.so` exists (line 1412) — re-injecting it from the lane would race that logic for no benefit. |
| `pixi.toml` | One task in `[feature.dev.tasks]`: `worker-cache-restore = "bash scripts/worker-cache-restore.sh"`. |
| `docs/modules/09-dev-toolchain.md` | §3: record the injection posture (live lane self-contained: builds bare-shell) + staging/bootstrap tier note. §4: script inventory row. Both name commands (C5). |

### Delete

None.

## Data Flow (build-time only)

Cold-clone live build, bare shell:

```
pixi install (one-time, human)
  → pixi run worker-cache-restore (one-time per staging-cold machine, needs proxy once)
      staging empty? → script prints the exact 12-source curl list + exits 1
      staging has files? → sha256 verify against the wrap files' source_hash
                         → copy into packagecache
  → cmake --preset live (or IDE preset button)
      polyorch_pixi_setup: resolves pixi env, exports prefix
      lane registration: live lane _laneenv carries CC/CXX/LD_LIBRARY_PATH
        (CC/CXX trail the strip and win; LD_LIBRARY_PATH is not in the strip,
        so it survives as-is; LIBCLANG_PATH needs no injection - PolyOrch
        derives it automatically when <bin>/../lib/libclang.so exists)
  → cmake --build --target talkservo-deploy-build
      cmake -E env (strip → lane env → PATH) → cargo → build.rs → meson
        meson: packagecache hits → no network
        clang++-23 via CXX → C++20 OK
        flatc via LD_LIBRARY_PATH → conda libstdc++ OK
      → out/live/bin/talkservo-server staged
```

Warm path (worker already built): packagecache is post-success-cleaned by build.rs, but
the build dir `out/cargo-target/live/debug/build/mediasoup-sys-*/out/` persists — cargo
skips build.rs entirely, restore script is a no-op (prints "worker already built" if the
`.a` exists — cheap idempotence check inside the script).

## Integration Points

- **PolyOrch rust API**: `polyorch_rust_set_env_vars(TARGET ...)` — existing surface, used
  today for the LINKER pin. No supplier change. Ordering vs the strip is documented
  supplier behavior (after-strip wins).
- **pixi face**: `polyorch_pixi_setup(ENVIRONMENT dev)` already exports prefix info; the
  injection reuses it rather than re-deriving paths. Exact variable names are read from
  PolyOrchPixiHelpers.cmake at implementation time (PREFIX_OUT/BIN_OUT contract) — no
  hardcoding of `.pixi/envs/dev` literals in cmake.
- **Wire contract (modules/02)**: untouched. This change cannot affect signaling.
- **Sfu trait (modules/03)**: untouched.

## Rust Specifics

None — no Rust source changes. The only Rust-adjacent surface is build.rs behavior
(accepted as-is, upstream-owned). Feature gates unchanged: live lane keeps
`sfu-mediasoup,embedded-web`; stub lanes unchanged and un-injected (they never run meson).

## Error Handling

- Restore script: every copy preceded by sha256 assert (matching the wrap's `source_hash`);
  mismatch = loud fail with the re-download curl line. Never copies an unverified file.
- Lane env: if `find_program(cc)` or the pixi prefix resolution fails, the existing
  REQUIRED failures fire at configure time (loud, same as today's LINKER pin path).
- meson failures keep failing the build loudly (no new swallowing).

## Testing Strategy

Per architecture §4 discipline — every claim names the command that proves it:

1. **Bare-env configure**: `env -i HOME=$HOME TERM=$TERM PATH=/usr/bin:/bin cmake --preset live ...`
   → configures, lanes registered (grep configure output for
   `talkservo lane talkservo-server-live ... + test`).
2. **Bare-env build**: same env → `cmake --build build/presets/live --target talkservo-deploy-build`
   → exit 0, `out/live/bin/talkservo-server` exists (this is Success Criterion 1).
   During the build, verify injection actually reached meson:
   `grep -l "clang++-23" out/cargo-target/live/debug/build/mediasoup-sys-*/out/build/meson-logs/meson-log.txt`
3. **Cache restore round-trip**: wipe packagecache (script's own hygiene covers post-success
   cleanup) → `pixi run worker-cache-restore` → packagecache repopulated, sha256 all pass;
   then delete the meson build dir and rebuild live → meson consumes cache, no github fetch.
4. **No-regression on stub/daily flow**: `pixi run test` (ctest -L "rust|web") green;
   `pixi run gate` green (ledger gates + clippy + fmt + deny).
5. **Presets round-trip**: `cmake --list-presets` shows all three; configure each once.
6. **MediaServo-portability note** (documentation only): record in modules/09 that the
   `_laneenv` injection pattern is the proven native-IDE-building primitive for the family,
   applicable to MediaServo's future native-server mode.

## Dependencies

None new. No cargo/npm deps. All mechanisms are repo-local scripts + existing PolyOrch API.
