# Server API — Living Reference

> Living reference (C2): mirrors the implemented `talkservo-server` surface.
> Update in place as the server evolves; plan-3 (web/SDK) consumes this file.
> Design truth stays in `docs/modules/02` (protocol) — this file records what
> the binary actually serves today. Updated: 2026-09-29 (plan-2).

## Transport

- **WS** `GET /ws` — upgrade; single JSON object per text frame.
- Frame limit: `MAX_FRAME_BYTES` (default 64 KiB) → oversize = close `1009`.
- First message must be `Join{v, jwt}` within 10 s of upgrade.
- Close codes: `4400 bad_version` · `4401 expired/invalid token` · `1009 frame limit`.

## REST

| Route | Response |
|-------|----------|
| `GET /healthz` | `200 {ok:true, version}` (compose/CI readiness) |

### Admin console (`/admin/api/*`, plan-5 D18)

> **wave-1 note (2026-10-19)**: audit log / room registry / command authorization / role tiers / intervention kit are ADJUDICATED (D23 + docs/plans/2026-10-19-alpha-wave1-trust-control.md) — this section pre-dates wave-1; implementation has NOT started.

Audit trail (plan-7 S1): `logs/audit.jsonl` under the instance dir (yaml `audit_log:` / env `TS_AUDIT_LOG` override; file 0600, dated rotation on day rollover). Closed snake_case event enum over the five-tuple shape (actor/action/target/changes[{key,old,new}]/reason); relayed: floor grant/taken/idle (incl. MediaDown/Timeout paths), join/leave, mode_change + mute_set with actor, token_issue, login_fail. `authz_denied` fires from S3 (ModeChange/MuteSet by a non-Dispatch sender = FloorDenied{unauthorized} + audit; Dispatch targets are unmutable by non-Dispatch — modules/01 Rule 10), `kick`/`force_release` arm S6. Writer is fire-and-forget off-thread; IO failures increment the `dropped` counter AND log `tracing::error` (never panic, never silent).

All routes ALWAYS mounted; fail-closed `503` while `TS_ADMIN_SECRET` unset. Auth: `Authorization: Bearer <admin JWT>` — HS256, `aud:"admin"`, NO room claim (structural audience isolation; `/ws` can never accept an admin token). Tiers (plan-7 S5): login mints with the account's role into the admin JWT (`role` claim, serde-default `admin` = pre-S5 tokens keep full power); accounts + rooms-registry WRITE paths require tier `admin` (operator gets 403 fail-closed); all read paths + tokens + monitor are operator-usable. Console role is INDEPENDENT of the signaling `role` (dispatch/field) and of mod-claims.

Credential sources: `init` bootstraps `etc/accounts.yaml` with user `admin` + **plaintext password** (sister ops shape — the file is 0600, gitignored, hand-editable, hot: edits take effect on the next login without restart); `POST /admin/api/accounts` mints/rotates rows as PBKDF2 `hash:` (API never writes plaintext; upsert clears any plaintext row so the new hash rules); logins accept either (`password:` constant-time compare, else PBKDF2 verify). `scripts/issue-admin-token.sh` stays as the ops escape hatch.

| Endpoint | Body / params | Response |
|---|---|---|
| `GET /admin/api/overview` | — | `{rooms, peers, uptime_s, backend}` |
| `GET /admin/api/rooms/live` | — (1s per-room fan-out timeout) | `[{room, mode, holders[], queue_len, peers[{id, role, mod_rooms[], granted, muted, connected_since_ms}], muted[], speaking[]} \| {room, stale:true}]` — renamed from `/admin/api/rooms` at plan-7 R1 (registry owns the clean name); the peer fields are the S6.2 enrichment the console Drawer reads |
| `POST /admin/api/login` | `{user, password}` | -> `{jwt, role}` (200; 401 wrong credentials; 503 = no credentials configured) — registry = `etc/accounts.yaml` (`[{user, hash, role}]`, program-maintained, atomic write, HOT: created accounts log in without restart; implicit `admin` from server.yaml `admin_password_hash:` until first materialization, carried over never stranded; `TS_ACCOUNTS_FILE` env overrides path) — TTL `TS_ADMIN_LOGIN_TTL_S` default 43200 s; 60/min rate shared with tokens; password never logged |
| `GET /admin/api/accounts` | — | -> `[{user, role}]` (hashes never listed) |
| `POST /admin/api/accounts` | `{user, password, role?}` (S5: `role` = `"admin"\|"operator"`, create-only - reset keeps stored tier) | -> 201 / 400 — hash minted server-side (`pbkdf2$iters$salt$dk`), never echoed back |
| `DELETE /admin/api/accounts/{user}` | — | -> 204 / 404 — existing stateless-JWT sessions survive until expiry (documented D22 limit) |
| `GET /admin/api/rooms` | — | -> registry rows `[{room, owner, mode?, max_peers?, max_queue?, locked}]` (R1: registry owns the clean name) |
| `POST /admin/api/rooms` | `{room, owner?, mode?, max_peers?, max_queue?, locked?}` | -> 201 (upsert) / 400 validation (name charset = token charset, ranges) |
| `DELETE /admin/api/rooms/{room}` | — | -> 204 = row removed AND live room evicted (members get 1000 Close, audit `leave` lines via S1 relay); 404 unknown/absent |
| `DELETE /admin/api/rooms/{room}/peers/{peer}` (S6 kick, console tier - operator OK) | optional `{reason}` (<=512, audit-only) | -> 204 / 404 (live room + member required); target socket closed, survivors get `peer_left{kind:"removed"}` (no reason/actor on wire - R2) |
| `POST /admin/api/rooms/{room}/peers/{peer}/force-release` (S6 recall) | optional `{reason}` (<=512, audit-only) | -> 204 / 404; injects the peer's own FloorRelease through the authorized path (grants shrink -> PIT-21 rebroadcast / idle); audit `force_release` carries actor+reason |
| `GET /admin/api/audit` | query `{before?,after?,room?,actor?,event?,limit? (1..=1000, default 100)}` | -> `{events:[{seq,ts_ms,event,room,actor,target,changes,reason}], dropped}` - newest-first seq-cursor page over the JSONL trail; 503 when audit is disabled |
`peer_left` carries additive `kind` (default `left`; `removed` = console kick/eviction - serde-default keeps old clients parsing).

| `POST /admin/api/tokens` | `{sub, room, role:"dispatch"\|"field", ttl_s?}` (ids: 1-64 `[A-Za-z0-9._-]`; ttl clamped `1..=TS_JWT_TTL_S`) | `{jwt, field_url, dispatch_url, participant_url}` (participant link added with the /room shell, 2026-10-10); base = `TS_PUBLIC_BASE` (scheme-inclusive; Host-header fallback is loopback-dev only). Rate: 60/min ⇒ 429. Trail: `token_issue` audit line + the `admin_token_issued` log (token value never in either) |

REST contract of record = this table (`openapi.yaml` deferred to Alpha external-API work with CI validation — plan-2 /healthz precedent, adjudicated 2026-10-08).

## Join handshake (J sequence, server→client order)

1. `Welcome{v:1, peer_id, turn_creds}` (TURN creds are empty stubs until the live
   host slice; real HMAC short-TTL creds per modules/06 land with it)
2. `RouterCaps{media_codecs}` (audio/opus mono, modules/04 pinned)
3. `PeerJoined{info, generation}` — **to others only**; the joiner gets the full list
4. `PeerList{peers, generation}`
5. `ServerSnapshot{payload, generation}` — role-scoped (D12): `field` payload has
   no queue; `dispatch` embeds it

Identity collision → `Error{code:"already_joined"}` (PoC policy; displacement
is an Alpha decision).

## Floor control

- `FloorRequest{priority, preempt}` → cooldown-gated (`FLOOR_REQUEST_COOLDOWN_MS`,
  default 500 ms) → within window: `FloorDenied{reason:"rate_limited"}`.
- Grant/deny/promote semantics = core `apply()` (modules/01 rules 1-9); broadcasts
  fan out per wire contract (`FloorGranted`/`FloorTaken`/`FloorDenied`/`FloorQueued`/
  `FloorIdle` — all carry `generation`).
- `FloorRelease` → auto-promote head; empty room → `FloorIdle`.
- `ModeChange{mode}` → R9 transitions; `MuteSet{peer,on}` (authorized roles; core
  trusts actor, server enforces claim checks before routing).

## Media plane (J-steps 3-7)

| Client → | Server → | Notes |
|----------|----------|-------|
| `TransportCreate` | `TransportInfo{ice,dtls,addrs}` | E10: `TRANSPORT_GUARDRAIL` (50) exceeded → `Error{media_capacity}` |
| `TransportConnect{dtls}` | — | required before Produce |
| `Produce{rtp_parameters}` | `ProduceOk{producer_id}` | created **paused** (D13 gating); `Produce` before `Connect` → `Error{bad_order}` |
| `Consume{producer_id}` | `ConsumeOk{producer_id, rtp_parameters}` | stub returns canned params |

E4: transport timeout → `MediaFailed{peer}` + `Error{transport_timeout}` (single
attempt at PoC; retry-1 lands with the live host).

## Resync / recovery

- `Resync` → fresh role-scoped `ServerSnapshot` (R1; stale-`gen` discard is client-side).
- E6/W-sequence: worker death → rebuild → `MediaRestart{room, reason}` broadcast
  → clients re-run J-3-7 (supervisor channel seam exists; live wiring = live host slice).
- E11: granted holder silent past `NO_RTP_WATCHDOG_MS` + `FLOOR_MEDIA_GRACE_MS` →
  `MediaFailed{peer}` + release + auto-promote (verified end-to-end over stub).

## Proactive pushes

- `TokenRefresh{jwt}` at `exp − 5 min` (per-member re-sign; modules/02 §1).
- `PeerLeft{peer, generation}` on disconnect (delta; survivors only).

## Configuration (env; full table with defaults: `config/sample.env`)

`TS_JWT_SECRET` (required, fail-fast) · `TS_JWT_TTL_S` · `TURN_TTL_S` ·
`FLOOR_MAX_HOLD_MS` (dispatch cap / `off` open) · `FLOOR_MEDIA_GRACE_MS` ·
`FLOOR_REQUEST_COOLDOWN_MS` · `HEARTBEAT_S` · `RTC_PORT_MIN/MAX` ·
`TRANSPORT_GUARDRAIL` · `NO_RTP_WATCHDOG_MS` · `ROOM_IDLE_TTL_S` · `MAX_PEERS` ·
`MAX_QUEUE` · `MAX_FRAME_BYTES` · `TS_BIND`.

## Not yet in the binary (honest gaps → next slice)

- `consume` server path (needs mediasoup-client deviceCaps exchange; the SDK
  media manager is ready, e2e exercises it once the browser lands).
- TURN credential issuance in `Welcome` (stubs today).
- `RoomIdleTtl` reaper task (config plumbed; loop lands with the host slice).
- E11 probing via AudioLevelObserver (paused-state inference only today).
- Canonical WS fixtures recording (cross-review F3; capture during browser
  e2e bring-up).
- Embedded web: feature `embedded-web` serves `web/dist` at `/` (LANDED —
  build web first).
