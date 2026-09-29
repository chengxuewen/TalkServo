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
