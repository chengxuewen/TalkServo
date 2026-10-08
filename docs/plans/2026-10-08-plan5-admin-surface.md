# Plan-5: Admin minimal surface (PoC-tail, D18) — v2 (review-adjudicated)

> **STATUS: COMPLETE 2026-10-08** (S0/S1/S2; gates: cargo 17 suites, lint clean, SDK 22, web 5, e2e signaling 3/3 + TS_ADMIN lane 4/4 incl. admin click-through). Live server-api.md registration done in S2 commit; modules/08 status note below.

> Design truth: `docs/modules/08-web-ui.md` §Admin surface. Slices S0→S1→S2; per slice implement→verify→commit. Inherits plan-2/3/4 global constraints (C1 English artifacts, gates before "done", lockfiles committed).
> **v2 = team-mode review outcome (Momus OKAY + Oracle NEEDS×20→all resolved as plan-text edits; 5 adjudication points user-adjudicated one-by-one 2026-10-08; summary in §Adjudication record below).**

## S0: pre-flight fixes (each its own commit, before any admin code)

- [x] 0.1 `fix(server)`: `room.rs::snapshot_for` ships `muted: vec![]` forever (line ~144 "expose below" never done) — populate from the existing FloorState mute set; integration assertion: mute → leave → rejoin(R1) → snapshot carries the mute (adjudicated #1A; fills an existing wire field — NO protocol change)
- [x] 0.2 `fix(server)`: main.rs restart fan-out (~53-63) holds `app.rooms` lock across `send().await` — clone-senders → drop lock → send (adjudicated #5A). This is the SAME discipline S1's AdminInspect must follow from day one; after this there is exactly one fan-out idiom repo-wide.

## S1: server — /admin/api + AdminInspect + bootstrap script

- [x] 1.1 `auth.rs`: `AdminClaims { sub, exp }` — **structurally NO room field** (structural audience isolation: /ws's two-pass decode fails on it even under misconfiguration); separate secret `TS_ADMIN_SECRET`, `aud:"admin"`; **startup guard: refuse boot if TS_ADMIN_SECRET == TS_JWT_SECRET**; unset secret ⇒ routes registered anyway, handler-guard returns 503 (NEVER conditional registration — the `/{*path}` SPA fallback would otherwise serve index.html 200 = fail-OPEN masquerading as closed)
- [x] 1.2 `RoomCommand::AdminInspect(oneshot<RoomReport>)` (ws.rs enum) — handler answers from RoomState only, zero SFU calls; `RoomReport { mode, holders, queue_len, peers[{id, connected_since_ms}], muted[], speaking }` where `speaking` = NEW small cache written by the existing WatchdogTick media_activity probe (one-line map store; adjudicated #3C)
- [x] 1.3 `admin.rs` + router wiring:
  - fan-out discipline (BLOCKER B1/B2): clone senders under lock, drop, then per-room `tokio::time::timeout(1s)`; timeout/send/recv error ⇒ that room `"stale": true` (tolerate reaper races; one wedged room must never hang the poll)
  - `GET /admin/api/overview` — rooms/peers counts, backend label (`media.backend()`), uptime (`App.started_at` NEW field, set in main.rs)
  - `GET /admin/api/rooms` — the fan-out above
  - `POST /admin/api/tokens` — body via **typed `Deserialize`** structs only (PIT-5 guardrail: no `Value::to_string` on ids ever); validation: sub/room ≤64 chars + URL-safe charset (room goes into a URL PATH), role ∈ {dispatch, field}, `ttl_s` clamped `1..=TS_JWT_TTL_S`; response links pinned to `${base}/f/${room}?jwt=…` / `/d/…` with URL-encoded room (matches web intake `?jwt=` — session.ts, verified: zero page changes needed); link base = `TS_PUBLIC_BASE` **scheme-inclusive, required for LAN deploys** (Caddy TLS ⇒ Host-fallback would mint http:// links = getUserMedia dead); Host fallback loopback-dev-only, documented
  - rate limit (adjudicated #4A): process-wide 60 issues/min, 429 beyond (precedent: floor-request cooldown knob); `tracing::info!` on `admin_token_issued` {sub,room,role,ttl} — token value NEVER logged (only audit trail D18 allows)
- [x] 1.4 `scripts/issue-admin-token.sh` (mirror issue-token.sh; note: run on the server host — clocks)
- [x] 1.5 config.rs: `admin_secret: Option<String>`, `public_base: Option<String>` in `from_env`; `for_test` keeps its signature (new fields `None` — 9 call sites ripple-free); sample.env rows for both + posture comments
- [x] 1.6 tests `crates/talkservo-server/tests/admin_api.rs`: 503-unset (NOT 200-index) / 401 missing / 401 wrong-secret / user-JWT-on-admin-route + admin-JWT-on-/ws both directions / issued token joins /ws / AdminInspect empty+active shapes / stale-room on a deliberately-wedged mailbox (send without draining → 1s timeout, response marks stale) / rate-limit 429 / validation rejects non-URL-safe room + oversized + role enum / snapshot-muted assertion via 0.1's path
- [x] 1.7 gates: `pixi run lint` + stub-lane workspace green; commit `feat(admin): out-of-band admin API — overview/rooms/tokens + AdminInspect (plan-5 S1)`

## S2: web — /a route (Monitor + Tokens)

- [x] 2.1 `web/src/pages/admin.tsx` + `/a` route (before the `*` redirect; SPA fallback already serves extensionless paths — verified): gate card (paste admin JWT → sessionStorage only, 401 ⇒ back), Sider (Monitor|Tokens), Monitor = stat cards + rooms table **with holder tally (reuse `.tally-ring`), muted chips, per-holder speaking indicator** (adjudicated #3C), 3s poll + fetch-error badge (stale flag rendered per room); Tokens = form → three copy buttons (`navigator.clipboard`, textarea fallback)
- [x] 2.2 `web/src/lib/adminApi.ts` (Bearer wrapper; 401 callback) — stays web-side: out-of-band REST, not SDK wire domain (D14 thin-view intact; D10 Electron wraps the same SPA)
- [x] 2.3 vitest: link-assembly (exact `?jwt=` shape), gate state transitions, stale/muted/speaking rendering (mock fetch)
- [x] 2.4 e2e stub-lane row (global-setup: TS_ADMIN_SECRET behind `TS_ADMIN` env flag, mirrors coturn pattern): open `/a`, paste bootstrap token, joined peer visible, issue token, **click-through issued field link reaches connected** (real production path, no test-only hooks)
- [x] 2.5 docs: `docs/reference/server-api.md` registers /admin/api (request/response shapes = contract of record) + plan deviation note `openapi.yaml deferred to Alpha external-API work (plan-2 /healthz precedent; CI validation lands with it)`; modules/08 status note; status.md; commit `feat(web): /a admin console — monitor (muted+speaking) + token issuance (plan-5 S2)`

## Global constraints (this plan)
- NO new runtime deps; NO SignalingMessage variants; NO media-plane change; **no wire additions — 0.1 fills an existing field only**
- fail-closed (503-by-guard); tokens never logged; audience isolation structural (no-room AdminClaims) + startup secret-divergence guard
- `TS_ADMIN_SECRET` / `TS_PUBLIC_BASE` physically separate from user-JWT secrets; sample.env documents all
- fan-out: exactly one idiom repo-wide (clone→drop→send+timeout)
- C1 English artifacts; each slice gated before commit; docs/README plan table updated at close

## Adjudication record (user-adjudicated one-by-one, 2026-10-08)
| # | question | decision |
|---|----------|----------|
| 1 | snapshot muted bug placement | A — plan-5 S0a, own commit |
| 2 | REST contract of record | B — server-api.md + deviation note; openapi.yaml at Alpha |
| 3 | Monitor columns | C — muted + per-holder speaking (watchdog cache) |
| 4 | tokens rate limit | A — 60/min, 429 |
| 5 | legacy lock-held fan-out | A — S0b pre-flight fix, single fan-out idiom |
Non-contested review fixes folded in without adjudication: BLOCKER B1 (clone-drop-lock fan-out), B2 (1s timeout + stale), C1 (always-registered 503), C2 (scheme-in-BASE posture); MAJOR A1 (structural AdminClaims + startup guard), C3 (concrete validation list). Open items: none.
