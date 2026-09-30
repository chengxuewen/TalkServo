#!/usr/bin/env bash
# Loopback netem (acceptance #8 harness) — MUST run in a dedicated root
# console, never over SSH through lo (review-focus pin: command leakage kills
# the session). Usage: netem.sh on|off
set -euo pipefail
case "${1:-}" in
  on)
    sudo tc qdisc add dev lo root netem delay 40ms 10ms loss 12%
    echo "netem ON: lo 40ms±10ms, 12% loss"
    ;;
  off)
    sudo tc qdisc del dev lo root 2>/dev/null || true
    echo "netem OFF"
    ;;
  *)
    echo "usage: $0 on|off" >&2
    exit 1
    ;;
esac
