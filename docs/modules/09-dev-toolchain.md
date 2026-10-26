# Dev Toolchain — pixi + bootstrap scripts

> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md). Convention C2.
> Decision: D11 (2026-09-28). Evidence: MediaServo `pixi.toml`/scripts audited same day; adopted/adapted/rejected matrix below.

## 1. Core posture: Linux-native-first

TalkServo's development host is Linux x86_64 — which is also the mediasoup worker's native platform. Therefore:

- Daily loop builds/tests `talkservo-server --features sfu-mediasoup` **natively in the pixi sandbox**; no Docker prerequisite.
- Docker is demoted to two roles: a **clean-environment parity image** (ubuntu:22.04, built and inspected by hand — there is no hosted runner, D26) and the **deployment artifact**. `docker-cargo.sh` exists as an ops tool, not a dev entry point.
- macOS remains a supported *check target*, but nothing runs it automatically (D26): the command is `cargo check --workspace --no-default-features --features talkservo-server/stub-media`, and it is checked by whoever holds a Mac. MediaServo's Docker-required-everywhere posture was an artifact of macOS developer machines; we do not import that constraint into the dev path.
- The inherited rule files `.agents/rules/common/platform.md` / `docker.md` carry a status note pointing here (their macOS guidance applies only if a macOS dev machine enters the picture).

## 2. Adopted / adapted / rejected matrix

| MediaServo asset | Verdict | TalkServo form |
|------------------|---------|----------------|
| pixi.toml workspace+features model | adopt | skeleton in §3 |
| bootstrap.sh → pixi.sh two-stage entry | adopt | trimmed: linux + mac paths only, no Jetson/WSL detection |
| `scripts/_common.sh` (logging, pixi guard, `set -euo pipefail`) | adopt | keep the pkill/`|| true` discipline (PIT-15 lineage) |
| MESON_ARGS/MESON unset trick (conda-vs-tasks.py buildtype clash) | adopt verbatim | same mediasoup-sys build path — the pitfall is universal, comment cites the sister PIT |
| `[activation]` env injection (MESON/NINJA/LIBCLANG_PATH/PKG_CONFIG_PATH) | adopt | |
| cargo-deny / tarpaulin / clippy -D warnings tasks | adopt | config files ported (`deny.toml`, `tarpaulin.toml`, `clippy.toml`) |
| docker-cargo.sh | adapt | clean-environment parity wrapper, run by hand (D26); not part of bootstrap |
| nodejs inside pixi | **new** (sister lacked it) | node>=22 in base deps → reproducible `web/` Vite builds |
| bindings tasks (build-c/test-cxx/test-py/abi-drift/parity) | reject-for-now | trigger: Beta, when `bindings/` gains real code (D8) |
| GStreamer/flatbuffers deps, imgui/vendor layers, Jetson aarch64 activation, *.bat windows suite, e2e brand/compat probes | reject | each needs a named TalkServo consumer first |
| .so.<MAJOR> dev symlinks | defer | Beta with bindings |

## 3. pixi.toml shape (LANDED 2026-09-29 — pixi.toml at repo root; deviations from this sketch: +pip/invoke/requests deps (mediasoup-sys build.rs drives the worker build through python invoke), +CC/CXX→conda clang++-23 (base env has no gcc; mediasoup-sys build.rs probes libstdc++ via CXX), +LD_LIBRARY_PATH=$CONDA_PREFIX/lib (conda-built flatc needs conda GLIBCXX at runtime — host libstdc++ too old))

```toml
[workspace]  name="talkservo"  platforms=["linux-64","osx-64","osx-arm64"]
[dependencies]
rust=">=1.85,<2"   python="3.12.*"   nodejs=">=22"
meson ninja clang libclang openssl pkg-config zlib ripgrep pyyaml
[feature.dev.tasks]
check="cargo check --workspace"           build="cargo build --workspace"
test ="cargo test --workspace"            lint ="cargo clippy --workspace --all-targets -- -D warnings"
test-sfu="cargo test -p talkservo-server --features sfu-mediasoup"
format="cargo fmt --all -- --check"       format-fix="cargo fmt --all"
audit="cargo deny check"                  coverage="cargo tarpaulin --workspace --out Html --out Lcov"
web-install="npm ci --prefix web"         web-build="npm run build --prefix web"   web-dev="npm run dev --prefix web"
run-server="cargo run -p talkservo-server" run-coturn="docker compose -f docker/coturn.yml up"
[feature.ci.tasks]
ci-check="cargo check --workspace"        ci-check-mac="cargo check --workspace --no-default-features --features talkservo-server/stub-media"  # review H-3: zero-backend fires compile_error — stub must be explicit
ci-lint / ci-test / ci-test-mediasoup(ubuntu image)
[activation] env = { MESON=…, NINJA=…, LIBCLANG_PATH=…, PKG_CONFIG_PATH=… }  # + MESON_ARGS unset comment citing sister PIT
[environments] dev=["dev"] ci=["ci"]
```

Workspace members at creation: `crates/{core,sfu,server}` only (D7); binding dirs absent until Beta (no empty shells).

## 4. Bootstrap flow

```text
new machine:  git clone → source bootstrap.sh     # detects/installs pixi (curl), pixi install, prints next step
              then: pixi run client-install && pixi run web-install   # npm provisioning per clone; the graph never installs
daily:        source pixi.sh                      # exec pixi shell -e dev (or `pixi run <task>` one-offs)
verify loop:  pixi run lint && pixi run test        # daily, minutes
              (a fresh clone needs BOTH pixi run client-install and pixi run web-install: talkservo-web-build
               runs npm run build only, whose tsc --noEmit also typechecks ../packages/client/src - missing deps
               surface as sh: 1: tsc: not found or Cannot find module mediasoup-client, and configure now warns
               per package when either node_modules directory is absent)
push-to-live: TS_LANE_LIVE=1 pixi run test-sfu   # mediasoup backend incl. the C++ worker: measured 1545s
clean-env parity: scripts/docker-cargo.sh test -p talkservo-server --features sfu-mediasoup   (ubuntu:22.04 image, by hand — no hosted runner, D26)
secrets:      scripts/scan-hardcode.sh            # path already referenced by security-hardening skill — implementation = this repo's grep-based scan
```

`scripts/` initial inventory at P0 (LANDED 2026-09-29: _common.sh bootstrap.sh pixi.sh scan-hardcode.sh docker-cargo.sh + Cargo.toml pixi.toml rust-toolchain.toml deny.toml clippy.toml tarpaulin.toml; docker/ compose files deferred to plan-2 T5): `_common.sh bootstrap.sh pixi.sh scan-hardcode.sh docker-cargo.sh` + `Cargo.toml pixi.toml rust-toolchain.toml deny.toml clippy.toml tarpaulin.toml .gitignore` deltas + `docker/` compose files. No cargo tasks may reference crates that do not exist — bootstrap must be idempotent on a crates-less checkout (check-guard, exit 0 with notice).

## 5. Decision hooks

> **D25 pointer (2026-10-19)**: the build ENTRY migrates to a PolyOrch CMake graph (submodule
> embed, Rust/Node/gate/install lanes — full blueprint + P1-P4 schedule in
> docs/plans/2026-10-19-plan8-polyorch-build-assembly.md). pixi remains environment
> single-source; this module's task tables stay current until their phase retires them.

- **PolyOrch supplier baseline (plan-11, 2026-10-23)**: the recorded gitlink is
  `f8f85db88240590e20dd910414a7ef3fd6d81982`; PolyOrch version is `0.7.1`.
  TalkServo embeds PolyOrch through the `PolyOrch::PolyOrch` sentinel and requires
  `INTERFACE_POLYORCH_VERSION >= 0.7.1`. The self-embed uses `EXCLUDE_FROM_ALL`, so
  PolyOrch's own example/test targets do not become TalkServo build targets.
  Normal Rust lanes use `polyorch_rust_test(... CTEST ...)` on PolyOrch `0.7.1`;
  the live server lane remains wrapper-gated because `TS_LANE_LIVE` is a run-time condition,
  and its ctest label remains `mediasoup;headless;slow`.
- **P2 state (plan-8, 2026-10-19)**: `build`/`test`/`web-build`/`web-test` are now graph
  forwarders (`cmake --build --target talkservo-*-build` / `ctest -L "rust|web"`);
  `check`/`lint`/`format`/`audit`/`test-sfu` stay direct-cargo TOOLCHAIN tasks (no graph
  equivalent for `cargo check`; clippy/deny are toolchain probes). Web-lane toolchain pins
  discovered in P2: `packageManager` field in both package.json files (corepack offline
  resolution - without it, corepack fetches `npm/latest` through the proxy and dies),
  conda `nodejs >=22,<23` (node 26 breaks jsdom: unimplemented getComputedStyle kills the
  AntD admin gate test), and the live mediasoup lane's `CARGO_TARGET_<triple>_LINKER` = conda
  `cc` (NOT the CMake CXX driver). Plan-11 review narrowed this pin: supplier CTEST has no
  lane ENV keyword, so ungated stub lanes are deliberately unpinned on both build and test
  to avoid rebuild thrash; the runtime-gated live lane keeps the pin in its hand-wrapped ctest
  path because `cargo test` relinks the bin and harnesses and `cc` inside pixi has a sysroot
  lacking `__libc_single_threaded` that mediasoup-sys static objects need.
- **P1.0 posture (plan-8)**: `polyorch_pixi_setup` runs WITHOUT `COPY_SCRIPTS` — our
  `pixi.sh`/`pixi.bat` stay repo-owned (subshell-entry semantics per §4 above; PolyOrch's
  template is a source-activate file, swapping it would change the daily ritual). Sister
  repos use COPY_SCRIPTS; we deliberately diverge here and record the choice.
- **P1.0 env**: `cmake >=3.25` added to the pixi host dependencies (PolyOrch embed floor,
  PolyOrch D39 equivalent — foreign MediaServo-sister ledger id, not a TalkServo decision); resolved in Pixi.lock 2026-10-19.

## 6. Lane target vocabulary and cache facts (plan-10, D27; landed 2026-10-22)

Rule, one line: **`talkservo-<object>-<verb>`**, verb in {`build`, `test`} for a lane, plus
`clean` wherever a deletable artifact actually exists (the cache groups, `web/dist`); object =
a lane `<crate>[-<config>]` or a scope `lanes[-<config>]`; config in {omitted, `stub`, `live`},
and a lane carries the config suffix **only** where that package has more than one in-graph
configuration. `stub|live` is deliberately the same word used by `TALKSERVO_PACKAGE_LANE`,
`talkservo.sh --lane`, the `e2e-stub`/`e2e-live` labels and `TS_E2E_FEATURES` — one vocabulary,
five surfaces. The `talkservo-` prefix is load-bearing, not decorative: CMake target names are
global to the project and this graph is built to be embedded in a superbuild (first-wins
sentinel), where a bare `lanes-clean` would collide with any sibling repo on the same pattern.

Inventory (18 targets, every one carrying an IDE `FOLDER`, asserted at configure time):

| lane | build | test | clean | cache group | ctest label |
|---|---|---|---|---|---|
| `talkservo-core` | -build | -test | — group clean | `out/cargo-target/stub` | `rust;headless` |
| `talkservo-sfu` | -build | -test | — group clean | stub | `rust;headless` |
| `talkservo-server-stub` | -build | -test | — group clean | stub | `rust;headless` |
| `talkservo-server-live` | -build | -test (`TS_LANE_LIVE=1`, else Skipped) | — group clean | `out/cargo-target/live` | `mediasoup;headless;slow` |
| `talkservo-web` | -build | -test | -clean (`web/dist`) | npm | `web;headless` |
| scope `lanes` | `talkservo-lanes-stub-build` (aggregate, carries `ALL`) | — | `talkservo-lanes-{stub,live}-clean` per group + `talkservo-lanes-clean` root | — | — |
| scope `gate` | `talkservo-gates-build` (runs `ctest -L gate`, not `ALL`) | — | — | — | — |
| scope `e2e-stub` | `talkservo-e2e-stub-build` (runs `ctest -L e2e-stub`, depends on the stub aggregate, not `ALL`) | — | — | — | — |
| scope `deploy` | `talkservo-deploy-build` (builds the configured `TALKSERVO_PACKAGE_LANE` handle and stages `out/<lane>/bin/{talkservo-server,oxmgr}`; lane = the install knob's configure value, not a per-lane button; runtime `init` stays in talkservo.sh, D25 red line) | — | — | — | — |

Four facts that cost measurement to learn, recorded so nobody re-pays them:

- **One handle per (package, config) — lib and bin are not two build units.** The helper takes
  exactly one artifact kind per handle, and `cargo build -p talkservo-server --bin …` compiles the
  lib unit in the same call (verified: two `rustc` invocations, `--crate-type lib` then `--crate-type bin`).
  The pre-plan-10 `talkservo-server` lib handle therefore tracked an artifact nothing consumed.
  A second handle for the same package appears only when something really consumes the other
  artifact — the reserved case is the D8 Beta C-ABI staticlib.
- **`ctest -L` is a partial regex.** A label named `rust-live` is matched by the daily
  `-L "rust|web"` filter, silently widening the fast lane into the slow one (measured: 5 entries
  instead of 4). The live label is therefore `mediasoup`; the ratchet is
  `ctest --test-dir out/cmake-build -L "rust|web" -N | grep "Total Tests"` → **4**.
- **Deletion is per cache group, not per crate.** `cargo clean -p talkservo-sfu --target-dir
  out/cargo-target/stub` is refused whenever that directory lacks a valid `CACHEDIR.TAG` (cargo
  aborts protectively to avoid deleting files it does not own - measured here: the stub group has
  `debug/`, `tmp/` and `.rustc_info.json` but no tag). The graph therefore offers only `rm -rf` of
  directories it owns outright; surgical per-package cleaning stays a shell command for whoever
  needs it, with that caveat attached (PIT-41). `add_custom_target` has no `FOLDER` keyword either,
  so IDE groups are set with `set_property(TARGET … PROPERTY FOLDER …)` and
  `talkservo_assert_target_folders()` fails the configure if any `talkservo-*` target lacks one (PIT-40).
- **The gate's cache is not the lanes' cache, on purpose.** Sharing `gate-clippy` with the live
  group was tried and reverted: after `cargo clean -p mediasoup-sys`, the gate command did not
  finish in 23 minutes in the shared dir, because the C++ worker rebuild lands in the clippy path
  either way. So `./target` (which once reached 33 GB) is clippy's own rmeta set, not duplication.
  Corollary for humans: a manual `cargo …` without `--target-dir` forks whichever directory the
  lanes are not using — for lane work, pass `--target-dir out/cargo-target/{stub,live}`.
- **There is no default goal any more, by decision.** `build all` used to depend solely on the
  embedded PolyOrch directory, so a bare `cmake --build out/cmake-build` built nothing and exited
  0; the stub aggregate now carries `ALL` (cold measured 79s incl. `web/dist`), and it excludes
  the live lane so the worker stays opt-in.
- **`scripts/gate-lanes.py --self-test` checks that Cargo members, CMake lane
  registrations, docs inventory, install handles, clean targets, and ctest labels remain
  one registry.

Tooling note (PIT-30 recurrence): plain `ninja -C out/cmake-build -t targets` prints NOTHING from a
shell without the pixi env on PATH, and a downstream `grep` then "passes" on the empty stream. Read
`out/cmake-build/build.ninja` instead, or prefix with `~/.pixi/bin/pixi run -e dev`.



D11 (this posture) · D5/D7 (default-features discipline + member gating) · D6 (mediasoup is why meson/ninja/clang exist at all) · platform.md/docker.md rule files annotated to defer to this module.

**Lockfile rule (review O-§3)**: `pixi.lock` commit is mandatory alongside `Cargo.lock` — conda-side meson/clang/openssl versions must not float across machines (that float is precisely the MESON_ARGS pitfall surface above).

## Delivery shell (D19, plan-6) — design truth

Two-layer model (sister-proven, no custom daemonizer): `talkservo.sh` outer shell (bash; build lanes stub|live, versioned-tar package, `build:deploy` one-step, `deploy` defaulting `--prefix out/<lane>` (user ruling 2026-10-09: the live instance lives at `out/live`; an out-of-repo prefix remains the option if clean-class wipes recur — PIT-16). Fresh init bootstraps `etc/accounts.yaml` (user `admin` + plaintext password, 0600, hot-editable — sister ops shape); existing registries are preserved, never re-rolled., compose pass-through up/down, dev `run`, `doctor`) + **facade subcommands inside talkservo-server** (`init/start/stop/restart/status/logs/monit/ps <dir>`; default dir = bin sibling when its `run/oxfile.toml` exists, else `.server/`) supervising via vendored **oxmgr** (3rdparty sha256 archive, PATH-injected, packaged into bin/). Instance dir = the portable deploy unit: `etc/server.yaml` (the ONE operator file — every non-secret incl. bind + front_port; precedence process-env > identity sidecar > yaml > defaults, B/B2 2026-10-09), `identity/` 0600 secrets only (idempotent), `etc/Caddyfile` + `run/oxfile.toml` (generated artifacts, zero secret copies). `run`-mode and systemd/compose paths untouched (zero breakage). Command stubs for absent artifacts (client/sdk/desktop) exit 2 with D8/D10/D15 pointers — no scaffold crates. Full surface: [../plans/2026-10-08-plan6-tooling-ship-shell.md](../plans/2026-10-08-plan6-tooling-ship-shell.md).
