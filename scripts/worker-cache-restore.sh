#!/usr/bin/env bash
# worker-cache-restore.sh — seed meson's subprojects/packagecache for mediasoup-sys
# from a persistent staging dir (native-build-env-hardening, 2026-10-27).
#
# Why: mediasoup-sys's build.rs deletes subprojects/packagecache after every
# SUCCESSFUL worker build (cargo-publish hygiene, upstream-owned), and meson then
# re-downloads the C++ subproject tarballs from github.com/wrapdb — hosts where those
# are unreachable bare-direct need this restore before a cold live build.
# Integrity: every staged file is sha256-verified against the wrap's source_hash
# before it is copied; nothing unverified ever lands in the cache.
#
# Usage:
#   pixi run worker-cache-restore        (via the pixi.toml task)
#   bash scripts/worker-cache-restore.sh # direct
# Env:
#   TALKSERVO_WORKER_CACHE  staging dir (default: ~/.cache/talkservo/meson-packagecache)
# Exit codes: 0 restored-or-already-cached; 1 staging empty/incomplete (actionable list printed)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_common.sh
source "$SCRIPT_DIR/_common.sh"

# NOTE: no global "worker already built" short-circuit here (audit finding 2026-10-27):
# the mediasoup-sys SOURCE tree in ~/.cargo/registry is SHARED across every target dir
# (out/cargo-target/live AND the clippy gate's ./target), so a built worker in one tree
# does not exempt another tree's cold build.rs from needing the cache. Per-file sha256
# checks below already make a re-run cheap and idempotent.


STAGING="${TALKSERVO_WORKER_CACHE:-$HOME/.cache/talkservo/meson-packagecache}"
SRC=$(ls -d "$HOME"/.cargo/registry/src/*/mediasoup-sys-0.14.2/subprojects 2>/dev/null | head -1 || true)
if [ -z "$SRC" ]; then
  die "mediasoup-sys source not found under ~/.cargo/registry/src/*/ — run a cargo fetch first"
fi

CACHE="$SRC/packagecache"
mkdir -p "$CACHE"

# --- hygiene: a half-extracted subproject dir (previous failed build) lacks the patch
# content and makes meson fail with "exists but has no meson.build" - drop stale extractions
# so meson re-extracts from the (verified) cache cleanly
for d in "$SRC"/*/; do
  case "$(basename "$d")" in
    packagefiles|packagecache) continue ;;
  esac
  if [ -d "$d" ] && [ ! -f "$d/meson.build" ] && ls "$d" >/dev/null 2>&1; then
    # a wrap-source dir without meson.build: patched subprojects (openssl/libuv) always
    # carry one - treat as stale extraction and remove
    if grep -q "directory = $(basename "$d")" "$SRC"/*.wrap 2>/dev/null; then
      rm -rf "$d"
      log "removed stale subproject extraction: $(basename "$d")"
    fi
  fi
done

fail=0
for wrap in "$SRC"/*.wrap; do
  # pull filename/url/hash triples straight from the wrap (source and patch pairs)
  while IFS=$'\t' read -r fname fhash furl; do
    [ -n "$fname" ] || continue
    dst="$CACHE/$fname"
    if [ -f "$dst" ] && echo "$fhash  $dst" | sha256sum -c --quiet >/dev/null 2>&1; then
      continue # already present and valid
    fi
    staged="$STAGING/$fname"
    if [ -f "$staged" ] && echo "$fhash  $staged" | sha256sum -c --quiet >/dev/null 2>&1; then
      cp "$staged" "$dst"
      log "restored: $fname"
    else
      warn "missing or hash-mismatched in staging: $fname"
      warn "  fetch once (needs working network): curl -L -o '$STAGING/$fname' '$furl'"
      mkdir -p "$STAGING"
      fail=1
    fi
  done < <(python3 - "$wrap" <<'PY'
import configparser, sys
cp = configparser.ConfigParser(strict=False)
cp.read(sys.argv[1])
if not cp.has_section('wrap-file'):
    raise SystemExit(0)
s = cp['wrap-file']
for kind in ('source', 'patch'):
    fn = s.get(kind + '_filename')
    if fn:
        print('\t'.join((fn, s[kind + '_hash'], s[kind + '_url'])))
PY
  )
done

if [ "$fail" -ne 0 ]; then
  die "staging incomplete - fetch the listed files (one-time bootstrap; see docs/modules/09), then re-run"
fi
log "packagecache seeded from $STAGING (all sha256 verified)"
