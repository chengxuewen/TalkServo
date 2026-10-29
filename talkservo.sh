#!/usr/bin/env bash
# talkservo.sh — delivery outer shell (plan-6 / D19): build lanes, package,
# deploy assembly, compose pass-through, doctor. Dev inner loop stays in
# `pixi run <task>`; instance lifecycle lives in talkservo-server itself
# (init/start/stop/status facade over oxmgr).
#
# usage: ./talkservo.sh <cmd> [args]
#   build   [--lane stub|live] [--debug]   cargo build the lane, stage out/<lane>/bin
#   build:deploy [--lane ...] [--prefix DIR]  build then deploy, one step (sister D266 shape)
#   package [--lane stub|live]             -> dist/talkservo-server-<ver>.tar.gz (versioned top dir)
#   deploy  [--prefix DIR] [--lane ...]    assemble instance: bin/ + init (default prefix out/<lane>; idempotent, never rotates secrets)
#   up|down [service...]                   docker compose pass-through (server + coturn)
#   run     [--lane stub]                  foreground dev server (thin pixi passthrough)
#   doctor                                 toolchain + vendored-oxmgr presence report
#   version | -h
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$ROOT"
LANE="live"
OXMGR_VER="v0.5.0"

ver() { sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1; }

usage() { sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'; }

# pixi activation (mediaservo.sh shape: command -v then ~/.pixi/bin; -h skips)
ensure_pixi() {
    if command -v pixi >/dev/null 2>&1; then PIXI_BIN="$(command -v pixi)"
    elif [ -x "$HOME/.pixi/bin/pixi" ]; then PIXI_BIN="$HOME/.pixi/bin/pixi"
    else echo "pixi missing — install first (docs/modules/09 bootstrap)" >&2; exit 1; fi
    export PIXI_BIN
}

# vendored oxmgr: verify-then-unpack, version-keyed cache, WARN on unknown
# arch (never a wrong-arch binary — sister scheme-Y discipline)
ensure_oxmgr() {
    local arch; arch="$(uname -m)"
    case "$arch" in
        x86_64)  local pkg="oxmgr-${OXMGR_VER}-x86_64-unknown-linux-musl.tar.gz" ;;
        aarch64|arm64) local pkg="oxmgr-${OXMGR_VER}-aarch64-unknown-linux-musl.tar.gz" ;;
        *) echo "WARN: no vendored oxmgr for $arch — using PATH if any" >&2; return 0 ;;
    esac
    local cache=".cache/oxmgr/${pkg%.tar.gz}"
    if [ ! -x "$cache/oxmgr" ]; then
        ( cd "3rdparty/oxmgr" && sha256sum -c "$pkg.sha256" >/dev/null ) || {
            echo "FATAL: oxmgr archive checksum mismatch — refuse to unpack" >&2; exit 1; }
        mkdir -p "$cache" && tar xzf "3rdparty/oxmgr/$pkg" -C "$cache"
    fi
    PATH="$PWD/$cache:$PATH"; export PATH
}

lane_features() { # $1 lane -> cargo feature spec for talkservo-server
    case "$1" in
        stub) echo "--no-default-features --features talkservo-server/stub-media" ;;
        live) echo "--no-default-features --features embedded-web,sfu-mediasoup" ;;
        *)    echo "unknown lane $1 (stub|live)" >&2; exit 1 ;;
    esac
}

cmd_build() {
    local prof="--release"
    while [ $# -gt 0 ]; do case "$1" in
        --lane) LANE="$2"; shift 2 ;;
        --debug) prof=""; shift ;;
        *) shift ;;
    esac; done
    ensure_pixi; ensure_oxmgr
    echo "build: lane=$LANE ${prof:-debug}"
    # plan-8 P4: the graph is the builder; this script selects the lane target,
    # stages the bin component, and passes the ops web/dist copy through.
    local bdir="out/cmake-build" cfg="Debug" tgt
    [ -n "$prof" ] && { bdir="out/cmake-build-release"; cfg="Release"; }
    case "$LANE" in
        live|stub) : ;;
        *)    echo "unknown lane $1 (stub|live)" >&2; exit 1 ;;
    esac
    # plan-10 T2: the graph target name is composed from the lane word, so this script and
    # the lane registry can no longer drift apart by one renamed handle.
    # plan-11 follow-up: the graph owns build + staging now. ONE button,
    # talkservo-deploy-build, follows the configure's TALKSERVO_PACKAGE_LANE (this
    # script passes -DTALKSERVO_PACKAGE_LANE="$LANE" right below), so the button and
    # the install face share one lane fact. The runtime `init` stays HERE (D25 red
    # line: instance lifecycle is not a build step).
    tgt="talkservo-deploy-build"
    # Freshness for a DEPLOY verb: ninja skips the cargo call when the staged artifact
    # exists and the command line is unchanged (PolyOrchRustHelpers:365 —
    # "re-invocation is cargo-fingerprint-bounded") — but ninja never ASKS cargo, so a
    # source edit can miss the binary entirely (measured 2026-10-28: build:deploy
    # shipped a pre-fix binary after a serde change; 1h of false debugging, PIT-47).
    # Touch-based busters were tried first and REJECTED: bumping workspace sources
    # re-runs mediasoup-sys's build.rs (no rerun-if declared -> cargo's conservative
    # default fires on anything), which re-downloads the C++ subprojects on a bare-CN
    # network. The direct pre-build asks CARGO instead: same manifest/features/target-dir
    # as the lane, so cargo's own (accurate) fingerprint recompiles exactly the changed
    # crates into the SAME tree the graph step consumes; the later ninja step then finds
    # fresh BYPRODUCTS and passes through. A no-change deploy costs one cargo check-grade
    # pass (seconds); a changed one pays only the changed units.
    # mediasoup-sys's build.rs wipes meson's packagecache on every re-run and
    # re-downloads the C++ subprojects from github/wrapdb — unreachable bare-direct
    # here. Re-seed the cache (sha256-verified) before ANY cargo call on this lane.
    "$PIXI_BIN" run -e dev bash scripts/worker-cache-restore.sh || true
    local lane_features="stub-media,embedded-web"
    [ "$LANE" = "live" ] && lane_features="sfu-mediasoup,embedded-web"
    local PIXI_ENV_BIN="$ROOT/.pixi/envs/dev/bin"
    # SAME env wrapper as the graph's cargo call (cmake -E env with the lane pins) —
    # a divergent pre-build env would flip-flop cargo fingerprints between this step
    # and the ninja step, re-running the mediasoup-sys build.rs on every deploy
    # (measured: direct pixi-env pre-build + graph build ping-ponged the worker
    # build twice before this was unified).
    # (the trio + linker pin below is the lane env from TalkServoCrates.cmake —
    #  kept literally in sync; a divergence flips cargo fingerprints per deploy)
    "$PIXI_BIN" run -e dev cmake -E env \
        --unset=RUSTFLAGS --unset=CARGO_ENCODED_RUSTFLAGS \
        --unset=CFLAGS --unset=CXXFLAGS --unset=CC --unset=CXX \
        PATH="$PIXI_ENV_BIN:/usr/bin:/bin" \
        CC="$PIXI_ENV_BIN/clang" \
        CXX="$PIXI_ENV_BIN/clang++-23" \
        LD_LIBRARY_PATH="$PIXI_ENV_BIN/../lib" \
        LIBCLANG_PATH="$PIXI_ENV_BIN/../lib/libclang.so" \
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/usr/bin/cc \
        NO_PROXY="*" no_proxy="*" \
        cargo build --package talkservo-server \
        --manifest-path crates/talkservo-server/Cargo.toml \
        ${prof} --bin talkservo-server \
        --features "$lane_features" --no-default-features \
        --target-dir "out/cargo-target/$LANE"
    "$PIXI_BIN" run -e dev cmake -B "$bdir" -G Ninja -DCMAKE_BUILD_TYPE="$cfg" \
        -DTALKSERVO_PACKAGE_LANE="$LANE" >/dev/null
    "$PIXI_BIN" run -e dev cmake --build "$bdir" --target "$tgt"
    # web dist rides the embedded lane (rust-embed baked; keep a copy for ops)
    [ -d web/dist ] && { rm -rf "out/$LANE/web"; mkdir -p "out/$LANE/web"; cp -r web/dist "out/$LANE/web/dist"; }
    if command -v oxmgr >/dev/null 2>&1; then
        echo "staged: out/$LANE/bin/talkservo-server + oxmgr"
    else
        echo "WARN: oxmgr not staged — vendored cache unresolved; start/stop/monit need it on PATH" >&2
        echo "staged: out/$LANE/bin/talkservo-server"
    fi
}

cmd_package() {
    local dist="dist"
    while [ $# -gt 0 ]; do case "$1" in
        --lane) LANE="$2"; shift 2 ;;
        --dist) dist="$2"; shift 2 ;;
        *) shift ;;
    esac; done
    local stage
    [ -x "out/$LANE/bin/talkservo-server" ] || { echo "build first (no out/$LANE/bin)"; exit 1; }
    ensure_pixi; ensure_oxmgr
    mkdir -p "$dist"
    stage="$(mktemp -d "$dist/.stage-XXXXXX")"
    local name="talkservo-server-$(ver)-$LANE"
    # plan-8 P4: full install (bin + dist components) stages the exact tree;
    # the tar/top-dir contract stays here.
    "$PIXI_BIN" run -e dev cmake --install out/cmake-build-release --prefix "$stage/$name" >/dev/null
    mkdir -p "$dist"; find "$dist" -maxdepth 1 -name "${name}.tar.gz" -delete
    tar czf "$dist/$name.tar.gz" -C "$stage" "$name"
    rm -rf "$stage"
    echo "packaged: $dist/$name.tar.gz (extract: tar xzf $name.tar.gz; top dir $name/)"
}

cmd_deploy() {
    local prefix=""
    while [ $# -gt 0 ]; do case "$1" in
        --prefix) prefix="$2"; shift 2 ;;
        --lane) LANE="$2"; shift 2 ;;
        *) shift ;;
    esac; done
    # default prefix = the staged delivery tree itself (sister convention:
    # out/<target> doubles as the instance dir); explicit --prefix elsewhere
    [ -n "$prefix" ] || prefix="out/$LANE"
    [ -x "out/$LANE/bin/talkservo-server" ] || { echo "build lane $LANE first"; exit 1; }
    ensure_oxmgr
    [ -e "$prefix" ] && { [ -d "$prefix" ] || { echo "$prefix exists and is not a dir"; exit 1; }; }
    mkdir -p "$prefix/bin"
    # same-path short-circuit: default prefix stages IN out/<lane> — never self-cp
    if [ "$(readlink -f "out/$LANE/bin/talkservo-server")" != "$(readlink -f "$prefix/bin/talkservo-server")" ]; then
        cp -f "out/$LANE/bin/talkservo-server" "$prefix/bin/"
    fi
    local ox; ox="$(command -v oxmgr || true)"
    if [ -n "$ox" ] && [ "$(readlink -f "$ox")" != "$(readlink -f "$prefix/bin/oxmgr")" ]; then
        cp -f "$ox" "$prefix/bin/oxmgr"
    fi
    # init is idempotent: existing identity/etc/run files are preserved
    "$prefix/bin/talkservo-server" init "$prefix"
    echo "deployed: $prefix — $prefix/bin/talkservo-server start $prefix"
}

cmd_doctor() {
    local fail=0
    echo "(bare-PATH view — meson/ninja/clang living inside the pixi env are fine)"
    for t in "pixi" "docker" "node" "caddy" "meson" "ninja" "clang"; do
        if command -v "$t" >/dev/null 2>&1 || [ "$t" = "pixi" ] && [ -x "$HOME/.pixi/bin/pixi" ]; then
            printf 'ok    %s\n' "$t"
        else printf 'MISS  %s\n' "$t"; fail=$((fail+1)); fi
    done
    ensure_oxmgr >/dev/null 2>&1 || true
    command -v oxmgr >/dev/null 2>&1 && echo "ok    oxmgr (vendored)" || { echo "miss  oxmgr (will matter for start/stop)"; fail=$((fail+1)); }
    [ -f Cargo.lock ] && echo "ok    lockfile" || { echo "miss  Cargo.lock"; fail=$((fail+1)); }
    echo "doctor: $fail issue(s)"; return "$fail"
}

case "${1:--h}" in
    -h|--help|help) usage ;;
    version) echo "talkservo $(ver)" ;;
    build) shift; cmd_build "$@" ;;
    build:deploy) shift; cmd_build "$@"; cmd_deploy "$@" ;;
    package) shift; cmd_package "$@" ;;
    deploy) shift; cmd_deploy "$@" ;;
    up|down) shift; ensure_pixi; "$PIXI_BIN" exec -- docker compose -f docker/compose.yml "$1" ${2:+--} ${@:2} ;;
    run) shift; ensure_pixi; "$PIXI_BIN" run cargo run -p talkservo-server --no-default-features --features talkservo-server/stub-media ;;
    doctor) shift; cmd_doctor ;;
    *) usage; exit 2 ;;
esac
