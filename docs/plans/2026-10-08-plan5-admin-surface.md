# Plan-5: Admin minimal surface (PoC-tail, D18)

> Design truth: `docs/modules/08-web-ui.md` §Admin surface. Slices S1→S2; per slice implement→verify→commit (incremental-implementation discipline). Inherits plan-2/3/4 global constraints (C1 English artifacts, gates before "done", Cargo.lock/pixi.lock committed with changes).

## S1: server — /admin/api + AdminInspect + bootstrap script

- [ ] 1.1 `auth.rs`: admin-token issue/verify branch (`aud:"admin"`, `TS_ADMIN_SECRET`); unset secret ⇒ admin route group answers 503 (fail-closed); reuse HS256 path
- [ ] 1.2 `room.rs`/`ws.rs`: `RoomCommand::AdminInspect(tokio::sync::oneshot::Sender<RoomReport>)` — `RoomReport { room, mode, holders, queue_len, peers: Vec<{id, connected_since_ms, speaking?}> }`; read-only, no state mutation
- [ ] 1.3 `admin.rs` (new module) + router wiring: `GET /admin/api/overview` (aggregate counts + uptime + backend label from app state), `GET /admin/api/rooms` (fan-out AdminInspect, join handles), `POST /admin/api/tokens` (validate body against wire caps ≤ MAX scope; issue user JWT; compose `field_url`/`dispatch_url` from a `TS_PUBLIC_BASE` env with request-Host fallback; NEVER log tokens)
- [ ] 1.4 `scripts/issue-admin-token.sh` (mirror issue-token.sh: sub, ttl; aud=admin, TS_ADMIN_SECRET)
- [ ] 1.5 tests `crates/talkservo-server/tests/admin_api.rs`: 503-unset / 401-missing / 401-wrong-secret / 401-user-JWT-on-admin-route (audience isolation both directions) / issued-token ⇒ /ws join succeeds / AdminInspect empty vs active shapes / tokens body validation
- [ ] 1.6 gates: `pixi run lint` Finished + `cargo test --workspace` stub lane all-green; commit `feat(admin): out-of-band admin API — overview/rooms/tokens + AdminInspect (plan-5 S1)`

## S2: web — /a route (Monitor + Tokens)

- [ ] 2.1 `web/src/pages/admin.tsx` + route in `main.tsx`: gate card (paste admin JWT → sessionStorage; 401 ⇒ back to gate), Layout+Sider (Monitor|Tokens), Monitor = stat cards + rooms table (tally ring on holders, queue count, 3s `setInterval` poll, error badge on fetch failure), Tokens = form → result panel (three copy buttons via `navigator.clipboard` with textarea fallback)
- [ ] 2.2 `web/src/lib/adminApi.ts` (fetch wrapper: Bearer inject, 401 ⇒ onUnauthorized callback)
- [ ] 2.3 vitest: link-assembly + gate transitions (mock fetch); keep dispatcher/field suites green
- [ ] 2.4 e2e stub-lane one row: server booted WITH `TS_ADMIN_SECRET` (global-setup adds it when lane flag set — default dev lane keeps unset ⇒ 503 matrix stays server-tested only); open `/a`, paste bootstrap token, assert rooms table reflects a joined peer, issue a token, click-through the issued field link reaches connected state
- [ ] 2.5 docs: modules/08 §Admin surface status note (living doc), acceptance/README row? (admin not an acceptance item — skip), status.md structure table + `admin-dashboard` absence note stays; commit `feat(web): /a admin console — monitor poll + token issuance (plan-5 S2)`

## Global constraints (this plan)
- NO new runtime deps (server or web); NO SignalingMessage variants (out-of-band only); NO media-plane change
- fail-closed posture; tokens never logged; admin JWT never accepted by /ws and vice versa (audience check in both verifiers)
- TS_ADMIN_SECRET stays physically separate from TS_JWT_SECRET (independent rotation; sample.env documents both)
- C1 English artifacts; each slice gated (lint/tests) before commit; `docs/README.md` plan table updated at close
