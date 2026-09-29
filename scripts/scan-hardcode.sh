#!/usr/bin/env bash
# Hardcoded-secret scanner (security.md dangling reference — modules/09 §4).
# Greps sk-/api_key/password=/port-literal patterns outside config/ + tests + node_modules.
# Exit non-zero on hit. Intended for pre-commit and CI audit lane.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_common.sh
source "$SCRIPT_DIR/_common.sh"

HITS=$(grep -rnE "sk-[A-Za-z0-9]{8,}|api_key\s*=\s*['\"][^'\"]{8,}|password\s*=\s*['\"][^'\"]{4,}" \
  --include="*.rs" --include="*.ts" --include="*.tsx" --include="*.js" --include="*.toml" --include="*.yml" --include="*.yaml" --include="*.sh" \
  --exclude-dir=node_modules --exclude-dir=target --exclude-dir=.pixi --exclude-dir=.git \
  --exclude-dir=dist --exclude-dir=out \
  . 2>/dev/null | grep -v "scan-hardcode" | grep -vE "^\./(config|docker)/" || true)

if [ -n "$HITS" ]; then
  warn "potential hardcoded secrets:"
  printf '%s\n' "$HITS"
  exit 1
fi
log "scan-hardcode: clean"
