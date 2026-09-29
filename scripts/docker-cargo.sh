#!/usr/bin/env bash
# CI-parity wrapper (modules/09 §4): run cargo inside ubuntu:22.04 + full mediasoup build chain.
# Ops tool, not a dev entry point (D11). Usage: scripts/docker-cargo.sh <cargo args...>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_common.sh
source "$SCRIPT_DIR/_common.sh"

IMAGE="talkservo-ci-parity:22.04"
DOCKER="$(command -v docker || true)"
[ -n "$DOCKER" ] || die "docker required for CI-parity runs (dev loop is native per D11)"

# Build the parity image once: ubuntu 22.04 + rustup stable + meson/ninja/libuv/ssl.
if ! $DOCKER image inspect "$IMAGE" >/dev/null 2>&1; then
  log "building CI-parity image (one-time, 15-30 min: mediasoup worker chain)"
  $DOCKER build -t "$IMAGE" - <<'DOCKERFILE'
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      curl build-essential pkg-config libssl-dev libuv1-dev meson ninja-build \
      python3 python3-pip git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /work
DOCKERFILE
fi

log "docker-cargo (CI parity): cargo $*"
exec $DOCKER run --rm -v "$PWD:/work" -w /work "$IMAGE" cargo "$@"
