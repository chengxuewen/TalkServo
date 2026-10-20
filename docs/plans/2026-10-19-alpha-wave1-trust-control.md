# Plan-7: Alpha wave-1 — Trust & Control (D23) — draft for review

> **STATUS: S1+S2 LANDED (2026-10-19; registry green incl. eviction-leave audit). S3 next.**
> Design truth: `docs/modules/08-web-ui.md` + `.agents/memorys/decisions.md` D23 (incl. UI annex + borrow annex).
> Sister precedent: plan-5 format (slices → gates → commit per slice).

---

## 0. Why this wave exists (one paragraph)

PoC proved the floor-control core AND ran past the whitepaper §11 roadmap (R1–R9 priority/
preempt/queue/mode-switching — planned for Alpha — are already core-tested). The remaining gap
is not arbitration but **trust**: nothing is recorded (audit = 0), authority is unchecked
(`let _ = actor;` — any field token can flip modes), roles are decorative (one tier), and admins
can see but not act (no kick/release/detail). D23 (2026-10-19, eight user-adjudicated rulings)
defines wave-1 as: **audit skeleton first, then room authority chain, then role tiers, then the
intervention kit** — so every later action auto-leaves evidence. Wave-2 (Listen + AdminEvent
push) is explicitly out.

## 1. Scope summary

| # | Item (D23 ruling) | Server | Web |
|---|---|---|---|
| 1 | Audit log (JSONL, Discord 5-tuple schema) | `audit.rs` writer + 8 event sites | `Logs` tab (read-only) |
| 2 | Room authority chain (mod-claims + actor check) | `auth.rs` claims + `room.rs` authz | Tokens form mod multi-select |
| 3 | Role tiers (admin/operator) | `admin.rs` guard branch | Accounts role select + Sider filter |
| 4 | Intervention kit (kick / force-release / detail) | 2 REST endpoints + AdminInspect ext | Monitor row-expand + Popconfirm + Drawer |
| 5 | Rooms registry (rooms.yaml + CRUD) | `rooms.rs` + RoomState config | `Rooms` tab |
| 6 | deny≠mute distinction (borrow: Mumble suppress) | 0 (FloorDenied exists) | field/participant copy |
| 7 | Pre-flight fixes (U1: dead code + lying comments — PARTIALLY DONE) | — | 4 files, done in session |

**Out of scope (explicit)**: Listen button, AdminEvent WS push (wave-2 batch); wire message
variants (claims change only); room hierarchy/parent links (ledgered, YAGNI); quotas surfacing
(Alpha+); recording (OQ-3); SOS/GPS (D9 deferral stands).

## 2. Slice order (adjudicated: audit-first so later slices auto-leave evidence)

```
S0 fixes (DONE in-session)          S5 role tiers
  │                                   │
  ▼                                   ▼
S1 audit skeleton ──────────────► S6 intervention kit (kick/release/detail)
  │  (later slices land events here)  │
  ▼                                   ▼
S2 room registry (rooms.yaml)    S7 UI five-tab split (R wires it all)
  │
  ▼
S3 authz chain (actor check)
  │
  ▼
S4 mod-claims (tokens + SDK)
```

Dependency notes: S1 before everything (evidence); S3 before S4 (check exists, then who);
S6 needs S3+S4+S1 (who may act + is it recorded); S7 is pure UI, doable after S5.

## 3. Slices in full

### S0 — pre-flight fixes (U1 ruling: immediate, NOT plan-gated)
- [x] 0.1 media.spec #12 `>= -1` tautology → real `micTruth().serverPaused === true` assertion
- [x] 0.2 field.tsx: dead `PREEMPT_PRIORITY` const + lying AudioCues header comment → honest stub note
- [x] 0.3 dispatcher.tsx: dead gain Slider (no onChange) + unused import → deleted
- [x] 0.4 floor.spec urlFor dead line → deleted
- [x] 0.5 talk.spec cross-client assertions (`Speakers (2)` on p1 AND f3; `mode: open` on p1) — PIT-21 rule
- [x] 0.6 admin.test 401-bounce (poll → 401 → gate + jwt cleared)
- [x] 0.7 playwright.config: retries:0 / forbidOnly / trace retain-on-failure (console-checklist §6-K)
- [ ] 0.8 status.md SDK count 16→22 (STALE — write back with ledger batch)
- Gates: tsc ✅ vitest 7/7 ✅ (in session); e2e + lint re-run at slice close.

### S1 — audit skeleton (server + writer; adjudicated: JSONL, Discord 5-tuple)
- 1.1 `crates/talkservo-server/src/audit.rs` (~120 lines): `AuditEvent` serde struct =
  `{ts, event, room?, actor?, target?, changes?, reason?}` —
  `event` = closed enum serialized snake_case (S1 landed deviation: numbering adds nothing for a local file; the borrowed property is the closed typed set) `{ floor_grant, floor_taken, floor_idle, mode_change, mute_set, kick,
  force_release, join, leave, token_issue, login_fail, authz_denied }` (numbered serde enums,
  Discord `AuditLogEvent` shape); `changes` = `[{key, old, new}]`; query = `?before=&limit=&
  room=&actor=&event=` (id-cursor paging). Writer: append-only `out/live/logs/audit.jsonl`,
  daily rotation by filename suffix, 0600, tokio `spawn_blocking` append (never on hot path).
- 1.2 Event sites (8): floor.rs outcomes relay (grant/taken/idle), mode_change, mute_set,
  join/leave (ws.rs), token_issue (admin.rs, extends the existing line), login_fail (login
  handler), authz_denied (lands with S3 — site created here, fired there).
- 1.3 `GET /admin/api/audit` (admin guard; read-only, cursor paging, filters) + tests.
- 1.4 Tests: audit module unit (serialize roundtrip, rotation edge); integration — P sequence
  produces grant+taken+idle lines; authz_denied placeholder fires in S3's tests.
- Diagram (event flow):
```
floor.rs apply() ──out vec──► ws.rs broadcast ──┬──► peer sinks (unchanged)
                                                └──► audit.rs append (fire-and-forget)
admin.rs login_fail ────────────────────────────────► audit.rs
admin.rs token_issue ───────────────────────────────► audit.rs
```
- Commit: `feat(server): audit skeleton — JSONL writer + /admin/api/audit (plan-7 S1)`

### S2 — room registry (rooms.yaml + per-room config)
- 2.1 `etc/rooms.yaml` (`[{room, owner, mode, max_peers, max_queue, locked}]`), admin REST CRUD
  `GET/POST/DELETE /admin/api/rooms-registry/{room}` (naming TBD in review — `/registry/` avoids
  colliding with the read-only rooms poll); file 0600 atomic write = accounts.rs idiom; hot-read.
- 2.2 `RoomState::new` consumes config (mode default replaces hard-coded Hybrid at room.rs:59,
  per-room limits replace global); **locked room ⇒ Join rejected** (wire `Error{reason}` — no
  new variant); delete room = kick-all via S6 path (UI confirm states it).
- 2.3 Tests: config parse/defaults, locked-join rejection, per-room limits, CRUD chain.
- Commit: `feat(server): room registry — per-room config + CRUD (plan-7 S2)`

### S3 — actor authorization (the `let _ = actor;` fix)
- 3.1 floor.rs: `ModeChange`/`MuteSet` gain real checks — actor must be `Role::Dispatch` (claims
  already carry role) OR hold a mod-claim (S4); denial emits `FloorDenied{reason: Unauthorized}`
  **+ audit `authz_denied`** (S1 site fires). RoomState checks at message intake (room.rs match),
  NOT inside core apply() — core stays a pure machine; it gains a `mode_change_authorized()`
  precheck helper only.
- 3.2 Tests: field-token mode_change ⇒ denied + audited; dispatch ⇒ allowed; 39→41 core tests.
- Diagram:
```
WS msg ──► room.rs intake ──► authorized?(claims, msg) ──no──► FloorDenied + authz_denied(audit)
                                   │yes
                                   ▼
                              core apply() (unchanged purity)
```
- Commit: `fix(server): enforce command authorization on ModeChange/MuteSet (plan-7 S3)`

### S4 — mod-claims (token carries room authority)
- 4.1 `auth.rs` Claims + `mod_rooms: Vec<String>` (serde default — old tokens stay valid);
  issue-token API + `TokenReq.mod_rooms`; `scripts/issue-token.sh` extra arg.
- 4.2 S3's check becomes: `role==Dispatch || claims.mod_rooms.contains(room)`.
- 4.3 SDK `types.ts` Claims mirror + wire tests; tokens UI multi-select (S7 wires form).
- 4.4 Tests: mod-token mode_change allowed on listed room, denied on unlisted; SDK roundtrip.
- Commit: `feat(server,client): moderator claims on tokens (plan-7 S4)`

### S5 — role tiers (admin/operator)
- 5.1 accounts role activated: `guard()` gains tier check — `Accounts` write paths +
  `rooms-registry` write paths require `role=="admin"`; operator 403 (fail-closed);
  login response carries role → sessionStorage.
- 5.2 Tests: **R4 permission-matrix truth table** (4 identities {admin, operator, mod-token,
  field} × {read endpoint, write endpoint, intervention endpoint} — one test fn, full
  combination assertions; admin_api 5→~14), role in login response.
- Commit: `feat(server): admin/operator tiers on admin API (plan-7 S5)`

### S6 — intervention kit (adjudicated: full three + Jitsi contract)
- 6.1 `DELETE /admin/api/rooms/{room}/peers/{peer}` — kick: reuses the Leave cleanup path
  (grant release + broadcast + media teardown); `POST .../force-release` — FloorEvent::Release
  server-injected; both accept optional `reason` (Jitsi contract).
  **R2 ruling**: reason lands in the AUDIT line only; the room broadcast is the
  EXISTING PeerLeft path clarified with an additive kind marker (`left:removed`,
  serde-default old value = `left`) — no reason, no actor over the wire (privacy +
  awareness; no new wire variant, old clients stay valid).
- 6.2 AdminInspect extension: per-peer `transport` summary (ice_state, bitrate, connected_since)
  — SFU stats via existing probe seam; stale-room discipline unchanged.
- 6.3 Tests: kick ⇒ peer's ws closes + floor release broadcast + audit line WITH reason +
  `left:removed` marker on the broadcast (other-peer mirror asserted, PIT-21 rule);
  force-release ⇒ FloorGranted rebroadcast (PIT-21 path); stats shape;
  **R4 audit fail-visible fault injection** (read-only path / simulated ENOSPC → append
  failure MUST increment a dropped-events counter + tracing::error — never panic the hot
  path, never silently drop); **R4 legacy-token compat** (token without mod_rooms field =
  empty set: Dispatch passes, field denied).
- Commit: `feat(server): intervention kit — kick/force-release/peer detail (plan-7 S6)`

### S7 — web: five-tab split + intervention UI (D23 UI annex)
- 7.1 `pages/admin/` directory: index (gate+layout+role-aware Sider) / monitor / rooms /
  tokens / accounts / logs, each ≤150 lines; **Accounts tab admin-only in Sider; server 403
  is the real gate**.
- 7.2 Monitor row-expandable peers + `[Force release]` `[Kick]` (Popconfirm + optional reason
  input) + peer-name Drawer (detail from 6.2); Rooms tab (CRUD table + modal; delete confirm
  names the kick-all consequence); Logs tab (cursor table + event/room/actor filters, manual
  refresh — no auto-poll); Tokens form gains mod-rooms multi-select.
- 7.3 *(moved to S7.5 by R3 ruling)* deny≠mute: field/participant pages render
  `FloorDenied` as "denied by dispatcher" distinct from muted state (data already in
  mirror events). SEPARATE SLICE — own commit after S7; droppable under schedule
  pressure without touching the S7 mainline (U2 acceptance preserved, cut isolated).
- 7.4 Tests: vitest +8 (Sider role filter, rooms CRUD form, logs render/filter, expand+
  confirm+drawer, mod multi-select, deny copy); e2e: admin kick → other peer's page shows
  departure (cross-client), logs line appears (audit e2e), locked room join rejected.
- Diagram (admin IA):
```
┌ Sider ──────────┐  Monitor │ Rooms │ Tokens │ Accounts(admin) │ Logs
│ role-aware tabs │     ▲        ▲        ▲          ▲             ▲
└─────────────────┘     │        │        │          │             │
                   expand+kick  CRUD   mod-select  role CRUD   cursor+filters
                   +drawer      modal  multi-req   (admin)     (manual refresh)
```
- Commit: `feat(web): admin five-tab console + intervention UI (plan-7 S7)`

## 4. Global constraints
- NO new runtime deps (audit = serde_json file; claims = jsonwebtoken existing); NO SignalingMessage
  variants; core `apply()` purity preserved (authz precheck outside core); wire stays v1
  (claims-shape change is additive-defaulted); C1 English; every slice gated (cargo test -p +
  tsc + vitest; e2e at S6/S7 close) before its commit; PIT-18 discipline (no parallel cargo).
- Audit lines NEVER contain tokens/passwords (D22 rule extends to audit).

## 5. Verification & acceptance (wave-1 exit)
- cargo: core 39→~43, server 79→~95; admin_api 5→~12; vitest 7→~15; e2e stub 5→8.
- The three audit E2E proofs: authz_denied appears when a field token pokes mode; kick line
  appears with reason; grant/taken/idle stream under talk.spec's existing scenario (zero new
  media fixtures — events ride the existing 3-client flow).
- Live-lane rerun of talk.spec (produce_ok ×4 stays green post-S3 — proves authz didn't break
  the granted path).

## 6. Borrow annex (U2 ruling — absorbed into slices above)
| Borrow | Source [V-code] | Lands in |
|---|---|---|
| Audit 5-tuple (enum event / actor / target / changes{key,old,new} / reason + cursor query) | Discord auditLog.ts | S1 |
| Kick contract (confirm + optional reason + room notification naming both parties + typed event) | Jitsi kick dialogs | S6 + S7 |
| deny≠mute distinction | Mumble suppress (negative space: nobody ships it) | S7.3 |

## 7. Adjudication record (user-adjudicated one-by-one)
Functional walkthrough 2026-10-19 (D23): room authority B / audit A / tiers A / kit A / order B.
UI walkthrough same day: five-tab+split A / expand+confirm+drawer A / Listen not-in-wave A.
Team-review walkthrough same day: U1 fix-now A / U2 borrow-all A / U3 split-testing A.
Plan-review walkthrough 2026-10-19: R1 rooms/live rename C / R2 kick reason+PeerLeft-kind C /
R3 deny-copy own slice S7.5 C / R4 gates+matrix+fail-visible+legacy-compat B (static grep gate
rejected). Docs-gap ruling: sync obligations #1 (modules/02 claims payoff) + #2 (modules/01
Rule 10) IN; #3-#6 NOT adopted (ledgered, not swallowed).
Open items: status.md 16→22 (S0.8, pending); Listen + AdminEvent = wave-2; rooms registry
endpoint naming confirmed at S2 start; Motorola Wave PTX + Discord role-matrix remain [U] in the
checklist dossier (no reachable source this session).

## 8. Docs sync obligations (per slice — gate item, not optional)
Accepted by user ruling 2026-10-19 (gaps #1 and #2 of the architecture-doc audit; #3–#6
explicitly NOT adopted — ledgered as open items, never silently swallowed):
- S1 → `docs/reference/server-api.md`: register `GET /admin/api/audit` (request/response
  contract row, plan-5 precedent) + audit file posture note.
- S2 → `docs/reference/server-api.md`: registry CRUD rows; rooms.yaml status line.
  **R1 ruling**: the management resource takes the clean name — `GET/POST/DELETE
  /admin/api/rooms[/{room}]` (registry CRUD); the live-state poll renames to
  `GET /admin/api/rooms/live`; six call sites follow (admin.rs routes, adminApi.ts,
  admin.test, admin.spec, server-api.md, modules/08) — one-time rename while no
  external API consumers exist (D14 unpublished).
- S3 → `docs/modules/01-floor-model.md`: NEW authorization rule in the rules table (Rule 10:
  ModeChange/MuteSet are authorized commands — actor must be Dispatch or mod-claim holder;
  denial = FloorDenied{Unauthorized}, audited). First doc mention of command authorization.
- S4 → `docs/modules/02-signaling-protocol.md`: **claims debt payoff** — the JWT claims shape
  (sub/room/role/aud/exp) has NEVER been written down (verified: zero hits this session);
  S4 writes the claims table FIRST (existing five fields), THEN adds `mod_rooms` with its
  additive-default compat note. api-interface-design discipline applies to the delta.
- S5–S7 → `docs/modules/08-web-ui.md`: rewrite the Admin surface section (three-tab text is
  stale) + architecture §4 acceptance gains the wave-1 rows (audit e2e trio + authz matrix).

## 9. Review checklist for the user (gate before implementation)
- [x] Slice order & scope (§2, §3) — accepted 2026-10-19 (docs-gap ruling: obligations #1+#2 in, #3–#6 out)
- [x] S2 endpoint naming — R1: clean `rooms` for registry, live poll → `rooms/live`
- [x] S6 reason + broadcast — R2: reason audit-only, PeerLeft additive kind marker
- [x] S7 deny≠mute — R3: own slice S7.5, droppable
- [x] Gates — R4: + permission matrix, audit fail-visible, legacy-token compat; no static grep gate

**STATUS: FULLY ADJUDICATED 2026-10-19 — ready for implementation on user's "go".**
