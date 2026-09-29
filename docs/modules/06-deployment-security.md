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

## Auth mechanism taxonomy & adoption ladder (2026-09-28)

| # | Mechanism | One-liner | TalkServo stage |
|---|-----------|-----------|-----------------|
| 1 | JWT (HS256, symmetric) | server-signed claims with expiry/aud, stateless verify | **PoC primary** (issue-token → Join, TTL 1h) |
| 2 | JWT (asymmetric RS/ES) | private-sign/public-verify; verifier keys distributable | Alpha-optional (if a separate issuance service appears) |
| 3 | API Key+Secret pair (kid) | paired keys, rotate/revoke without downtime | Alpha (rotation vehicle, review O-F1; LiveKit precedent) |
| 4 | PSK / static-secret HMAC | shared secret; short-term creds = HMAC(secret, timestamp) | **PoC: TURN creds** (static-auth-secret, 1h) |
| 5 | Username/Password | server stores argon2 hash; verification issues token (password never on the wire) | Alpha (admin issuance + tester-console login upgrade) |
| 6 | OIDC/OAuth2 (external IdP) | enterprise IdP (AD/Feishu/WeCom) issues; we only verify | Production (product decision) |
| 7 | mTLS (client certs) | mutual cert verification | Production (converged gateway face) |
| 8 | Device-embedded credentials | factory keys/certs + pairing flow | Beta+ (with C/C++ OEM consumers, D8) |
| 9 | WebAuthn/passkeys | phishing-proof public-key login | far-future (admin hardening) |

Ladder: PoC(1+4) → Alpha(+3 rotation, +5 accounts) → Beta+(+8 devices, +6 SSO) → Production(+7 gateway, +9 admin).

Adoption discipline: every mechanism converges to the SAME wire credential — a short-lived, room-scoped JWT (modules/02 gains no auth-specific messages); `TS_JWT_SECRET` and `TURN_SECRET` remain physically separate (independent rotation); a new mechanism requires a named consumer (D8 discipline) and only ever adds an issuance path, never a second verification surface.
