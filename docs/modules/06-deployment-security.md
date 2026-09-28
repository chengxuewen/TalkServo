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

- JWT HMAC shared secret via env (`TS_JWT_SECRET`), fail-fast validation at startup; room-scoped audience claim; **TTL 3600 s default**.
- **PoC token issuance**: `scripts/issue-token.sh` (pixi task `issue-token`) mints HS256 tokens from the env secret for dev/e2e — no public signing endpoint exists; replaced by admin-issued tokens at Alpha (surface list in research/ui/admin-ui-patterns.md).
- TURN credentials: HMAC short-TTL, **3600 s default**, delivered in `Welcome` (no static REST keys anywhere).
- Short-TTL TURN credentials server-signed per session (no static REST keys).
- `ModeChange`/mute overrides require elevated user priority; E8 is the wire-level attack surface — log + reject only, no bans (PoC).
- Floor-request rate limit (500 ms default) as anti-flood.
- SRTP mandatory (mediasoup default); no plaintext path.
- Full OWASP + secrets sweep before Beta via `security-hardening` skill.
