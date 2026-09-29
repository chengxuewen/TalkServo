# Media Pipeline & Network
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Pipeline, transports, TURN, weak-net harness

| Stage | Owner | Config |
|-------|-------|--------|
| Capture + 3A (web) | `getUserMedia` | `echoCancellation/noiseSuppression/autoGainControl` all true; native 3A = D4 FFI at Beta (Android AGC caveat: speakerphone loopback smoke test in acceptance) |
| Codec | Opus | **mono** 48 kHz fullband (review M-10: speech standard; halves idle uplink; stereo reserved for program-audio feeds), 20 ms, `useinbandfec=1` (RFC 6716 §2.1.7 + RFC 7587 fmtp), DTX off — moot: non-holders producer-**paused** (D13); Router mediaCodecs pinned |
| Transport | mediasoup WebRtcTransport | `enableUdp + enableTcp`; candidate preference **UDP → TURN-TLS/443 → ICE-TCP** (4G truth; direct ICE-TCP often blocked; compose opens TCP 40000-40100 as secondary — talktome precedent); ICE-Lite = no ICE restart, drop→close→F/W re-handshake; ports 40000-40100 configurable; coturn flags (`use-auth-secret`/`static-auth-secret` env/`realm`) pinned in `docker/coturn.yml` at P0-1 |
| NAT | coturn | short-TTL HMAC-signed turn creds delivered in `Welcome` (no static secrets in page); host/srflx/relay all exercised |
| Loss | receiver-side | browser stack NACK/PLI + libopus fec hidden decode — verify empirically, don't rebuild |
| Weak-net harness | `tc netem loss 12%` | A/B FEC on/off, 60 s; pass line: `concealmentEvents` share drops ≥30% (getStats-based, quantitative); also used for P-latency measurement |

Floor↔media coupling (**D13 gating**, supersedes M1 wording): publish once, server **pauses non-granted producers** — uplink gated by grant, not just relay (review M-2: privacy/mobile-data/battery); `FloorGranted{grants,gen}` resumes granted producers + their consumers for every other peer; `FloorIdle`/`FloorTaken` re-pauses; in `Open` mode all unmuted producers run; audibility is Router state — no renegotiation, one control round-trip (pause/resume diff per modules/03).
