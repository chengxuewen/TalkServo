#!/usr/bin/env bash
# Two-stage bootstrap entry (modules/09 §4): detect pixi → install if missing → pixi install
# → print next steps. Idempotent on a crates-less checkout (exit 0 with notice).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_common.sh
source "$SCRIPT_DIR/_common.sh"

if ! pixi_present; then
  log "pixi not found — installing to ~/.pixi (curl installer)"
  curl -fsSL https://pixi.sh/install.sh | bash
  export PATH="$HOME/.pixi/bin:$PATH"
fi
PIXI="$(pixi_cmd)"

log "pixi ($($PIXI --version)) present — resolving environment"
if [ ! -d crates ]; then
  warn "no crates/ yet — pixi install still safe (toolchain-only env)"
fi
$PIXI install

log "bootstrapped. Next steps:"
log "  source pixi.sh          # enter dev shell"
log "  pixi run check          # workspace check (needs crates/, lands with P0-1 Task 2)"
log "  pixi run test           # workspace tests"
exit 0
