#!/usr/bin/env bash
# Daily entry (modules/09 §4): drop into the pixi dev shell.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_common.sh
source "$SCRIPT_DIR/_common.sh"
exec "$(pixi_cmd)" shell -e dev
