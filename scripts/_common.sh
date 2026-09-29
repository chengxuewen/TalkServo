#!/usr/bin/env bash
# Shared shell helpers for TalkServo scripts (modules/09 §4).
# Discipline inherited from MediaServo edit-safety: set -euo pipefail; pkill needs || true.
set -euo pipefail

log()  { printf '[talkservo] %s\n' "$*"; }
warn() { printf '[talkservo] WARN: %s\n' "$*" >&2; }
die()  { printf '[talkservo] FATAL: %s\n' "$*" >&2; exit 1; }

pixi_present() {
  command -v pixi >/dev/null 2>&1 || [ -x "$HOME/.pixi/bin/pixi" ]
}

pixi_cmd() {
  if command -v pixi >/dev/null 2>&1; then pixi; else "$HOME/.pixi/bin/pixi"; fi
}
