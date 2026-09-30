# Gap Review — SDK + Web lane (packages/client, web/)

> **FROZEN** — internal audit snapshot, 2026-09-30. Evidence-based review of the
> plan-3 TS surface at the gap-closure commit. Never back-written; superseding
> fixes get new dated entries. Scope: `packages/client/src/*.ts`,
> `packages/client/tests/*.ts`, `web/src/**`, `web/e2e/*`, `web/tests/*`.
> Method: full read of every in-lane file + hand diff of 12 wire variants
> against `crates/talkservo-core/src/wire.rs` + server-side cross-checks
> (`ws.rs` join/close paths, `media.rs` Consume/ConsumeOk pairing).

---

## Protocol correctness (focus 2)

**Wire drift: NONE.** Hand-checked variants (TS union ↔ serde): `join{v,jwt}`,
`floor_request{priority,preempt}`, `floor_granted{grants,generation}`,
`floor_idle{generation,reason: string|null}` (serde `Option` → `null` ✓),
`welcome{v,peer_id,turn_creds}`, `token_refresh{jwt}`, `media_failed{peer}`,
`consume_ok{producer_id,rtp_parameters}`, `server_snapshot{payload,generation}`
with the `payload`-tagged flatten (field/dispatch) — all match, snake_case
throughout. `DenyReason` and `FloorMode` literals match `rename_all = "snake_case"`.

- **F1 — VERIFIED OK: gen-discard semantics are `<`, not `<=`.**
  `store.ts:38,42,50,58,62,67,71,75,83` — `msg.generation < m.generation` drops
  strictly-stale only; equal generations re-apply idempotently (safe: every
  equal-gen reducer is a pure overwrite). Matches D16 "receivers drop stale" and
  `core.test.ts` "drops stale events (gen < seen)".

- **F2 — VERIFIED OK: auth-expired is truly terminal.**
  `signal.ts:153-158` sets `closedByUs = true` *before* emit/reject; the
  subsequent server close (`ws.rs` sends Close 4401 after the error frame) hits
  `onclose` (`signal.ts:168-173`) where `!this.closedByUs` skips
  `scheduleReconnect`; the scheduled callback re-checks the flag
  (`signal.ts:226`) — double-guarded. Test `core.test.ts:142-156` covers it.

- **F3 — MEDIUM: `already_joined` is retryable forever (infinite reconnect loop).**
  `signal.ts:148-152` rejects the connect promise but does **not** set
  `closedByUs`. Server behavior (`ws.rs:176-186`): error frame then handler
  `return` → socket closes → `onclose` → `scheduleReconnect()` → next join →
  `already_joined` again. Any scenario where the identity is registered
  elsewhere (second tab with the same JWT, zombie half-open socket) puts the
  client on a permanent ≤30s-capped reconnect treadmill that can never succeed.
  **Fix:** treat `already_joined` like auth-expired (terminal: set
  `closedByUs = true`, surface a distinct event so the UI can prompt), or stop
  retrying after N consecutive `already_joined` codes.

## Session/state races (focus 1)

- **F4 — HIGH (BLOCKING): StrictMode double-mount breaks the connection —
  the "StrictMode safe" claim in `session.ts:2-3` covers creation, not connect.**
  `ensureSession` is genuinely idempotent (create-or-get, `session.ts:48-94`),
  but the page effects are not: `field.tsx:37-44` / `dispatcher.tsx:31-38` have
  no cleanup and call `connectSession` on every effect run; in StrictMode dev
  the effect fires twice before any WS event lands, and the stale
  `status.connected === false` guard (closure dep is `[room]` only) does not
  help. Consequences, traced end-to-end:
  1. `TalkServoClient.connect()` (`index.ts:65-77`) unconditionally creates a
     second `SignalTransport` and overwrites `this.signal`; the first (healthy)
     transport is orphaned but still subscribed to events.
  2. Second join → server replies `already_joined` and closes → second
     transport enters the F3 reconnect storm; `void connectSession(...)` has no
     `.catch` → **unhandled promise rejection** every cycle.
  3. All subsequent sends route through `this.signal` = the dead second
     transport (`readyState !== OPEN` → `signal.ts:181-185` silently drops) —
     **the PTT button silently does nothing in dev** while receive-side events
     still flow through the orphaned first transport (masking the breakage).
  Production builds don't double-invoke effects, which is why e2e (prod build)
  is green. **Fix (one guard, root cause):** make `connectSession` idempotent in
  `session.ts` — store a `connecting`/`connected` flag on `RoomSession` and
  early-return, or expose a disconnect/cleanup from the effect. Optionally have
  `TalkServoClient.connect()` close a previous non-null `this.signal` first.

- **F5 — VERIFIED OK (with one latent hole): stable snapshots + notify discipline.**
  `session.ts`: `mirror` is only ever *replaced* (`session.ts:75`), so
  `useMirror`'s getSnapshot is reference-stable between notifications.
  `statusCache` is invalidated in `notify()` (`session.ts:118-121`) **before**
  listeners fire, and `notify` is the only mutation point that changes
  `connected/selfId/events` — invariant holds for the current call graph.
  React 18 concurrent tearing is not possible: every store mutation + notify
  happens synchronously inside one WS event turn, and `useSyncExternalStore`
  re-checks the snapshot after a concurrent render anyway. Server snapshot is a
  stable constant. **Latent hole:** `dropSession` (`session.ts:107-113`)
  deletes the session but neither invalidates `statusCache` nor notifies — a
  drop + re-`ensureSession` of the same key would serve the stale cached status
  forever. `dropSession` currently has zero callers, so this is dead-path today;
  fix when wiring it up: `invalidateStatus(key); notify(key);` inside
  `dropSession`, and delete the cache entry in `ensureSession`'s create branch.

## Media manager (focus 3)

- **F6 — MEDIUM: `pendingConsumes` leak + stale-pairing race.**
  `media.ts:205` registers `{producerId → peer}` and sends `Consume`; if the
  server answers with `error` (the `media_error` path in
  `crates/talkservo-server/src/media.rs:88-97` sends `Error`, never
  `ConsumeOk`), the entry stays forever — no timeout, and `teardown()`
  (`media.ts:256-262`) does not clear the map. Worse than the bounded leak:
  after `media_restart` → `teardown`, a *late* `ConsumeOk` for a pre-restart
  request still matches (`media.ts:131-138`) and `instantiateConsumer` opens an
  audio handle for a producer identity that no longer exists post-rebuild — a
  ghost handle that persists until the next grant sweep (if the peer is no
  longer granted, `state` handling releases it; if it is, it lingers as a dead
  line). **Fix:** clear `pendingConsumes` in `teardown()` (one line), and
  optionally ignore `consume_ok` arriving within N ms of a teardown.
  Overwrite semantics (`Map.set`) keep the leak itself bounded — noted, not the
  issue.

- **F7 — VERIFIED OK: AudioHandle idempotency + release coupling.**
  `AudioHandleImpl.release/pause/resume` are flag-guarded idempotent
  (`media.ts:292-308`); `releasePeer` emits once and removes from the pool
  (`media.ts:268-275`); grant-drop, `media_failed`, `media_restart`, and
  `transportFailed` all route to release/teardown and are covered by
  `media.test.ts:55-114`. `consumeGranted` skips peers with live handles, so
  grant sweeps don't duplicate consumers.

- **F8 — LOW: `MediaManager.stack` is dead weight.** `media.ts:108,111` stores
  the `MediaStack` but nothing ever calls `load/createSendTransport`; the only
  media exchange implemented rides raw wire (`sendRaw`). Keep the DI seam only
  when the mediasoup-client slice lands, or drop the field until then.

## e2e quality (focus 4)

- **F9 — VERIFIED OK: assertions are wire-coupled where it matters.** Roster
  convergence, `mode: exclusive`, `Floor granted. You may speak.` /
  `Floor held by chief` (aria-live derives from `mirror.grants`), tally-ring
  holder placement, and queue/gen rendering all require real server
  `PeerList`/`FloorGranted`/snapshot traffic; a server that accepted joins but
  emitted no floor/roster events fails the suite. Two soft spots, both
  backstopped: (a) `HOLDING` text is local UI state (`field.tsx:153`) — passes
  with a broken server, but the coupled assertions in the same test fail first;
  (b) the `gen \d+` assertion (`floor.spec.ts:104`) also matches initial
  `gen 0`, but everything it summarizes is already asserted concretely above.
  The `.first()` on Preempt (`floor.spec.ts:87`) is order-agnostic by accident:
  every Preempt button routes through the *dispatcher's* session
  (`dispatcher.tsx:167-181`), so roster order (server `members: HashMap`,
  non-deterministic iteration in `room.rs:117`) cannot flake the click.

- **F10 — MEDIUM: cross-test identity-reuse race + no retry budget.**
  `floor.spec.ts` reuses room `e2e-room` and subs (`alpha`, `chief`) across
  tests; `context.close()` and the server processing the WS close are
  asynchronous, so the next test's `join` for the same identity can land while
  the old socket is still registered → `already_joined` → SDK never shows
  `connected` → flake. `playwright.config.ts` sets `workers: 1` (good) but no
  `retries`, and the server's disconnect reaping is the only de-dup path.
  **Fix:** unique rooms per test (`e2e-room-${Date.now()}`) or unique subs per
  test — cheapest and removes the shared-state premise entirely; `retries: 1`
  in CI as a seatbelt.

- **F11 — LOW: `diag.spec.ts` is debug scaffolding that runs in the suite.**
  No assertions, a hard `waitForTimeout(3000)`, console dumps
  (`diag.spec.ts:13-14`) — pure suite tax (and the only hard sleep in the
  lane; `floor.spec.ts` itself is sleep-free, all waits are
  `expect(...).toBeVisible` with timeouts, which is correct). Gate it behind
  `test.skip(!process.env.TS_DIAG)` or move it out of `testDir`.

- **F12 — LOW: fixed port 8090 in `global-setup.ts:9`.** A stale server from a
  crashed prior run makes the healthz gate pass against the *wrong* process
  (old secret → token 401s). `TS_BIND` from env or a port probe before spawn
  would harden it. Minor for a local harness.

## C1 / housekeeping (focus 5)

- **F13 — VERIFIED OK: C1 clean.** `grep -rnP '[\x{4e00}-\x{9fff}]'` over
  `packages/client/src tests`, `web/src`, `web/e2e`, `web/tests`,
  `web/playwright.config.ts` → zero hits (only `node_modules` third-party
  locales, out of scope). All UI strings English.
- **F14 — INFO: coverage 78.9% vs the 80% bar** (`testing.md`). Closest gaps
  are the untested error branches this review flags (F3, F6) — fixing those
  with tests closes the gap as a side effect. Non-blocking.
- **F15 — INFO: dead regex alternative.** `floor.spec.ts:93`
  `/Floor idle\. You may speak|Floor held by chief/` — the UI never renders
  "Floor idle. You may speak" (`field.tsx:50-55` renders "Floor idle." without
  the suffix); first alternative is dead. Harmless; prune for honesty.
- **F16 — INFO: `NativeWebSocketAdapter.readyState` is event-cached**
  (`signal.ts:48-62`); a `send()` between a local `close()` and the close event
  can hit a stale OPEN and throw uncaught from `ws.send`. Edge; wraps only
  browser sockets; note for the media slice.
- **F17 — INFO (positive): fixture replay** (`fixtures.test.ts`) feeding the
  server-recorded transcript through the same reduce path is the strongest
  drift guard in the lane — better than any static diff gate.

---

## Verdict

Blocking: F4 (StrictMode double-connect → dead send path + reconnect storm in
every dev session; one-guard fix in `connectSession`). F3/F6/F10 are real but
shippable follow-ups.

**sdk-web lane: NEEDS (1 blocking)**
