# Media Pipeline & Network
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Pipeline, transports, TURN, weak-net harness

| Stage | Owner | Config |
|-------|-------|--------|
| Capture + 3A (web) | `getUserMedia` | `echoCancellation/noiseSuppression/autoGainControl` all true; native 3A = D4 FFI at Beta (Android AGC caveat: speakerphone loopback smoke test in acceptance) |
| Codec | Opus | 48 kHz, 2ch, 20 ms, `useinbandfec=1`, DTX off; Router mediaCodecs pinned to this list |
| Transport | mediasoup WebRtcTransport | `enableUdp + enableTcp` fallback (4G); ICE-Lite server; ports 40000-40100 (configurable range) |
| NAT | coturn | short-TTL HMAC-signed turn creds delivered in `Welcome` (no static secrets in page); host/srflx/relay all exercised |
| Loss | receiver-side | browser stack NACK/PLI + libopus fec hidden decode — verify empirically, don't rebuild |
| Weak-net harness | `tc netem loss 12%` | A/B FEC on/off, 60 s; pass line: `concealmentEvents` share drops ≥30% (getStats-based, quantitative); also used for P-latency measurement |

Floor↔media coupling (M1): publish always, relay by grant — audibility is Router pause/resume state, zero renegotiation per turn.
