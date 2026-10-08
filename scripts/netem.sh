#!/usr/bin/env bash
# Impairment harness (acceptance #8/#3) — MUST run in a dedicated root
# console, never over an SSH session that routes through lo (review-focus
# pin: commands would self-disrupt). Modes:
#   on     — full loopback impairment (delays EVERYTHING incl. WS; coarse)
#   media  — SINGLE-LEG media impairment (D17/PIT-8): prio qdisc + filter
#            matching UDP with source port in the mask-aligned RTC range
#            40960-41215 (server->client downlink leg only; run the server
#            with RTC_PORT_MIN=40960 RTC_PORT_MAX=41215). TCP/WS and the
#            uplink leg stay clean — one-arm A/B without the lo cascade.
#   off    — remove the qdisc
set -euo pipefail
case "${1:-}" in
  on)
    tc qdisc add dev lo root netem delay 40ms 10ms loss 12%
    echo "netem ON: lo 40ms +/-10ms, 12% loss (everything)"
    ;;
  media)
    tc qdisc add dev lo root handle 1: prio
    tc qdisc add dev lo parent 1:2 handle 20: netem delay 40ms 10ms loss 12%
    # protocol=UDP(17) at IP proto field; L4 src port u16 mask 0xff00 value
    # 0xa000 at offset 20 == 40960-41215 (server media ports -> client leg)
    tc filter add dev lo parent 1: protocol ip prio 1 u32 \
      match ip protocol 17 0xff match u16 0xa000 0xff00 at 20 flowid 1:2
    echo "netem MEDIA single-leg: downlink UDP src 40960-41215 only (server needs RTC_PORT_MIN=40960 RTC_PORT_MAX=41215)"
    ;;
  off)
    tc qdisc del dev lo root 2>/dev/null || true
    echo "netem OFF"
    ;;
  *)
    echo "usage: $0 on|media|off" >&2
    exit 1
    ;;
esac
