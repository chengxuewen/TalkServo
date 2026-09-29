# Dev Toolchain — pixi + bootstrap scripts

> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md). Convention C2.
> Decision: D11 (2026-09-28). Evidence: MediaServo `pixi.toml`/scripts audited same day; adopted/adapted/rejected matrix below.

## 1. Core posture: Linux-native-first

TalkServo's development host is Linux x86_64 — which is also the mediasoup worker's native platform. Therefore:

- Daily loop builds/tests `talkservo-server --features sfu-mediasoup` **natively in the pixi sandbox**; no Docker prerequisite.
- Docker is demoted to two roles: **CI-parity image** (ubuntu:22.04 build matrix) and **deployment artifact**. `docker-cargo.sh` exists as an ops tool, not a dev entry point.
- macOS remains a supported *CI check target* (`--no-default-features`, stub-media) — MediaServo's Docker-required-everywhere posture was an artifact of macOS developer machines; we do not import that constraint into the dev path.
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
| docker-cargo.sh | adapt | CI-parity wrapper; not part of bootstrap |
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
daily:        source pixi.sh                      # exec pixi shell -e dev (or `pixi run <task>` one-offs)
verify loop:  pixi run lint && pixi run test && pixi run test-sfu
CI parity:    scripts/docker-cargo.sh test -p talkservo-server --features sfu-mediasoup   (ubuntu:22.04 image)
secrets:      scripts/scan-hardcode.sh            # path already referenced by security-hardening skill — implementation = this repo's grep-based scan
```

`scripts/` initial inventory at P0 (LANDED 2026-09-29: _common.sh bootstrap.sh pixi.sh scan-hardcode.sh docker-cargo.sh + Cargo.toml pixi.toml rust-toolchain.toml deny.toml clippy.toml tarpaulin.toml; docker/ compose files deferred to plan-2 T5): `_common.sh bootstrap.sh pixi.sh scan-hardcode.sh docker-cargo.sh` + `Cargo.toml pixi.toml rust-toolchain.toml deny.toml clippy.toml tarpaulin.toml .gitignore` deltas + `docker/` compose files. No cargo tasks may reference crates that do not exist — bootstrap must be idempotent on a crates-less checkout (check-guard, exit 0 with notice).

## 5. Decision hooks

D11 (this posture) · D5/D7 (default-features discipline + member gating) · D6 (mediasoup is why meson/ninja/clang exist at all) · platform.md/docker.md rule files annotated to defer to this module.

**Lockfile rule (review O-§3)**: `pixi.lock` commit is mandatory alongside `Cargo.lock` — conda-side meson/clang/openssl versions must not float across machines (that float is precisely the MESON_ARGS pitfall surface above).
