# Design: entry-flow-wave

Read [proposal.md](proposal.md) first (what/why/scope). This document is the how.

## Architecture

```
 ADMIN CONSOLE                          JOIN PAGES
┌─────────────────────────┐   issue    ┌──────────────────────────────┐
│ Rooms tab               │──────────▶│ /dispatch /room /field       │
│  row: [Enter][CopyLinks]│  (existing │  │                            │
│  │          │           │   endpoint)│  │ WS connect (jwt from query)│
│  │ mint via issue     │            │  ▼                            │
│  │ + window.open      │            │ session bridge (signal.ts)    │
│  ▼                     │            │  close code → failure copy    │
│ adminApi.issue(sub?) ──┼────────────┼─▶ 4010 expired / 4410+reason  │
└─────────────────────────┘            │    / reconnect (unchanged)    │
                                       └──────────────────────────────┘
        │ 1. empty sub?
        ▼
 tokens handler (admin.rs)
   gen_subject(): "u-" + 6 base36; retry ≤8 on collision
   collision domain = live seats ∪ registry rows
        │ 2. auth::issue (unchanged)
        ▼
 room-scoped JWT {sub: "u-a3f2k9", room, role}  ← D23 chain unmodified
```

One server-side seam (the issue default), two client surfaces (Rooms rows, join failure
states). The wire, the JWT claims, the floor machinery, and the D23 authority chain are
deliberately outside the change boundary.

## Files to Touch

### Create

| File | Purpose |
|---|---|
| `web/src/pages/admin/entryActions.tsx` | Row-action cell: Enter + Copy links handlers. Enter = `api.issue({room, role:"dispatch"})` (no sub → server generates) → `window.open(jwt dispatch url)`. Copy = three URLs joined by `\n` → clipboard with textarea fallback. Extracted to its own file so rooms.tsx stays table-plumbing and the vitest can mount the cell alone. |
| `web/src/components/joinFailure.tsx` | Shared failure card keyed by close code: EXPIRED(4010)/REMOVED(4410+reason)/DISCONNECTED. No "apply to join" affordance (lobby deferred, D29-#2). No retry for the terminal two; DISCONNECTED keeps the session bridge's own reconnect. |

### Modify

| File | Change |
|---|---|
| `crates/talkservo-server/src/admin.rs` | `tokens` handler: if `req.sub.is_empty()` → `gen_subject(&app, &req.room)` (helper ~15 lines: loop 8×, `"u-" + rand 6 base36`, occupied check = room seat map + `rooms.yaml` row names + previously issued-but-live subjects visible to the actor). Generated sub flows into the existing `TokenResp.sub` — response shape unchanged. |
| `web/src/pages/admin/rooms.tsx` | Row actions: append `<EntryActions room={r.room} />` cell (Edit/Delete untouched). |
| `web/src/lib/session.ts` or `packages/client/src/signal.ts` | Error object gains `closeCode` + `reason` passthrough on terminal closes (classification exists; only the fields were dropped on the floor). SDK-side change mirrors into web's copy per the drift-test pattern. |
| `web/src/pages/{participant,field,dispatcher}.tsx` | Render `<JoinFailure error={...} />` on terminal failure; remove any generic error text it replaces. |
| `web/tests/admin.test.tsx` | Raise outbound-body ratchet baseline 3→4 (Enter's issue call asserts method+path+body incl. **no sub field**); copy-text assertion (three URLs `\n`-joined); failure-tier copy assertions. |
| `web/e2e/admin.spec.ts` | One row: kick with reason from the console → participant page shows the reason copy (uses existing kick path + admin/ws). |
| `docs/modules/08-web-ui.md` | Admin tab section: entry-actions row; new "join failure states" table (close code → copy → action). |
| `crates/talkservo-server/src/admin.rs` tests | `gen_subject` unit tests: format/charset (`u-` + `[0-9a-z]{6}`), collision avoid (registry row pre-seeded), uniqueness across calls. |

### Delete

None.

## Data Flow

**Enter button** (the loop that must close in one click):

```
click Enter → adminApi.issue({room, role:"dispatch"})        [sub ABSENT]
  → POST /admin/api/tokens (admin JWT in header)
  → tokens(): req.sub empty → gen_subject() → "u-7g2k9x" (unoccupied)
  → auth::issue(sub, room, dispatch) → TokenResp{sub, jwt, urls...}
  ← urls.dispatch_url = ".../dispatch/<room>?jwt=..."
window.open(urls.dispatch_url)  → dispatch page connects via existing bridge
```

**Failure surface** (what the participant sees):

```
WS close 4010 → JoinFailure(EXPIRED): "link expired — ask the operator for a new one" — no retry
WS close 4410 → JoinFailure(REMOVED): "you were removed from the room" + reason verbatim — no retry
other closes → no UI change; session bridge auto-reconnects as today
```

User-facing copy language follows the join pages' existing convention (the implementer
reads the current page strings and matches them; English shown above is the working
default — C1 governs the artifact language, the page language governs the runtime strings).

## Integration Points

- **Issue endpoint contract**: `TokenReq.sub` becomes semantically optional; the struct
  field was already `String` (not `Option`) — empty-string means "generate". `valid_id`
  keeps rejecting empty for OTHER callers? No: the handler branches BEFORE validation
  (empty → generate → validated value); explicit invalid subs still 400. Response shape
  untouched → SDK `issue()` types unchanged.
- **adminApi (web)**: `issue()` already sends the full form; the no-sub call passes
  `{room, role}` — if the client type marks `sub` required, relax to optional there only.
- **signal/session bridge**: terminal close classification exists in
  `packages/client/src/signal.ts` (R7); web's `lib/session.ts` mirrors it. Touch the
  web copy; the SDK change rides the existing drift-test pattern only if the error
  object shape is part of the SDK's public surface (it is — `TalkServoClient` errors) —
  add fields additively.
- **Testing ratchet**: `web/tests/admin.test.tsx` grep-baseline rises 3→4 in the same
  commit as the new write path (testing.md rule 1).

## Rust Specifics

- `gen_subject` lives in `admin.rs` beside `valid_id` (same trust boundary); uses the
  crate's existing rand facade if present, else `getrandom` via the workspace's existing
  dependency set (jsonwebtoken/rand are already tree members — no new deps).
- Occupied check reads: room actor seats if the room is live (`app.rooms` map, try_lock
  cheap), plus registry rows (`crate::rooms::load`). No new locks held across await.
- No changes to `talkservo-core` (floor/wire untouched); no new feature gates.

## Error Handling

- Generation exhaustion (8 collisions — practically impossible): 500 with
  "subject generation failed" (loud, server-side log includes the room; operator sees it
  in Logs tab).
- Clipboard failure (permissions/insecure context): textarea fallback; final failure →
  the button shows a transient "copy failed" tooltip (no silent success — PIT-1 family:
  never claim what didn't happen).
- JoinFailure for UNKNOWN terminal codes falls into DISCONNECTED tier (never blank).

## Testing Strategy

1. **Rust unit** (`admin.rs` tests): generated format matches `^u-[0-9a-z]{6}$`; two
   calls differ; registry-seeded collision is avoided; explicit invalid sub still 400.
2. **vitest** (`admin.test.tsx`): Enter asserts POST /admin/api/tokens body has room+role
   and NO sub; Copy links asserts clipboard text == three URLs joined; JoinFailure
   renders expired/removed/reason/disconnect tiers from fake close codes.
3. **e2e** (`admin.spec.ts`): kick-with-reason row → participant page shows reason copy;
   (existing admin click-through keeps guarding the console).
4. **Gates**: `pixi run lint` (clippy clean on the new branch), `pixi run test`
   (rust+web), ratchet baseline check in the same suite.
5. **Manual browser pass** (verification-before-completion): real Enter click opens a
   connected dispatch tab; expired-link join shows the copy (mint with ttl=1).

## Dependencies

None new (rand/getrandom already in the tree; antd Popover not needed after ruling A;
clipboard via existing pattern).
