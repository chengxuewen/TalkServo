# Platform Constraints

> **TalkServo calibration (D11, 2026-09-28)**: the macOS-dev + Docker-server posture below is inherited from MediaServo's
> macOS reality. TalkServo's primary dev machine is Linux x86_64 — mediasoup builds and tests **natively** there.
> Docker remains for CI-parity runs and deploy images; the macOS workflow applies only if a macOS contributor joins.
> See `docs/modules/09-dev-toolchain.md`.


> Split from [constraints.md](constraints.md) per D202 (OpenCode config optimization).
> This file: OS/platform-specific constraints for TalkServo development.

## Platform Constraints



### macOS Development — Host/Client Native, Server Docker
- **`talkservo-core` / `talkservo-client` (when it exists, Beta)**: build natively anywhere — no mediasoup dependency. `talkservo-sfu`/`talkservo-server` with `sfu-mediasoup` need Linux x86_64 (native here; Docker on macOS contributors' machines).
- **Server (`talkservo-server`)**: Compile and run via Docker when `sfu-mediasoup` feature is enabled. The server binary and `cargo check` work natively on macOS, but mediasoup integration requires a Linux container.
- Use `docker compose up -d` for the server dev container. See `docs/modules/09-dev-toolchain.md`.

### mediasoup Only Builds on Linux x86_64
- mediasoup's C++ Worker (compiled via meson/ninja) is a **Linux x86_64-only** native binary. It does not build on macOS ARM64 or Windows.
- **Ubuntu 22.04 LTS is the recommended base** (mediasoup upstream uses it for prebuilt binaries, widest glibc compatibility).
- Dockerfile uses `ubuntu:22.04` + rustup (stable), CI uses `ubuntu-22.04` for test-mediasoup job.
- **macOS workflow**: `cargo check --features sfu-mediasoup` works (checks Rust bindings), but `cargo build` or `cargo test` with `sfu-mediasoup` fails. Full compilation and testing require a Linux environment.
- **Docker workflow**: The `dev` container image (rust:stable-bookworm + meson) provides the full mediasoup build environment.
- **CI**: The `test-mediasoup` job runs on `ubuntu-latest` only (see `.github/workflows/ci.yml`).

### CI: test-mediasoup Runs on ubuntu-latest Only
- `.github/workflows/ci.yml` (lands at P0-1) will define `test-mediasoup` with `runs-on: ubuntu-latest`. It installs meson, ninja-build, libuv1-dev, and libssl-dev before running `cargo test -p talkservo-server --features sfu-mediasoup`.
- The `check` and `test` jobs do run on both `ubuntu-latest` and `macos-latest` (for workspace-level validation without mediasoup features).

## macOS-Specific Gotchas

| Gotcha | Detail |
|--------|--------|
| mediasoup build fails | C++ Worker requires Linux + meson. Use Docker on macOS. |
| `cargo test --features sfu-mediasoup` fails on macOS | Runs fine on ubuntu-latest CI. macOS can only `check`. |
| Docker Desktop slow | Volume mounts 3-5x slower than native. Use cargo-cache volume. |
| First Docker build takes 15-30 min | mediasoup C++ Worker + Rust deps from scratch. |
| localhost WebRTC fails without STUN | ICE needs explicit candidates — even for loopback. |
| Cargo.lock drift | Always commit `Cargo.lock` alongside `Cargo.toml` changes. |

## See Also

- [constraints.md](constraints.md) — Git commit rules
- [docker.md](docker.md) — Docker + network constraints
- `docs/modules/development/docker-workflow.md` — Docker dev workflow details
