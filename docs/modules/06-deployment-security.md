# Deployment & Security
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Deployment modes

| Mode | Shape | Stage |
|------|-------|-------|
| Vertical slice | one binary supervises its worker; coturn container alongside | **PoC (this spec)** |
| Split services | signaling process(es) + SFU pool, shared nothing, floor events over internal RPC | Alpha |
| Edge/cloud | regional SFU clusters, K8s+Helm, multi-worker routers (OQ-4) | Production |
| Converged | + SIP/RTP gateway boundary (RelayBackend) | post-Beta |

## 2. Security posture (PoC-relevant)

- **Transport security (acceptance-blocking)**: page + WS over TLS — Caddy or axum-rustls (PoC self-signed accepted; browsers kill `getUserMedia` without secure context — MDN); `wss://` mandatory outside LAN.
- **Trust statement (D6 consequence)**: mediasoup's C++ worker terminates DTLS-SRTP per transport — **the server operator holds plaintext audio of every joined peer** (no E2EE/SFrame upstream). Accepted for PoC dispatch trust model; E2EE = OQ-10 before any external hosting.
- JWT HMAC shared secret via env (`TS_JWT_SECRET`), fail-fast validation at startup; room-scoped audience claim; **TTL 3600 s default**.
- **PoC token issuance**: `scripts/issue-token.sh` (pixi task `issue-token`) mints HS256 tokens from the env secret for dev/e2e — no public signing endpoint exists; replaced by admin-issued tokens at Alpha (surface list in research/ui/admin-ui-patterns.md).
- TURN credentials: HMAC short-TTL, **3600 s default**, delivered in `Welcome` (no static REST keys anywhere).
- Short-TTL TURN credentials server-signed per session (no static REST keys).
- `ModeChange`/mute overrides require elevated user priority; E8 is the wire-level attack surface — log + reject only, no bans (PoC).
- Floor-request rate limit (500 ms default) as anti-flood; PoC accepts per-IP connect/join flood as open risk (single-tenant LAN/VPS test) — one axum middleware at Alpha (review O-F6).
- Config keys (defaults, source of truth for P0-1 sample config): `TS_JWT_TTL_S=3600 TURN_TTL_S=3600 FLOOR_MAX_HOLD_MS=45000(dispatch)/off(open) FLOOR_MEDIA_GRACE_MS=2000 FLOOR_REQUEST_COOLDOWN_MS=500 HEARTBEAT_S=30 RTC_PORT_MIN=40000 RTC_PORT_MAX=40100 TRANSPORT_GUARDRAIL=50(arbitrary,measure) NO_RTP_WATCHDOG_MS=2000`.
- SRTP mandatory (mediasoup default); no plaintext path.
- Full OWASP + secrets sweep before Beta via `security-hardening` skill.
