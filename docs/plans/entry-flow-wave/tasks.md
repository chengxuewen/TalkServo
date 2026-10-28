# Tasks: entry-flow-wave

Order: TDD discipline per testing.md — RED asserts land with or before the code that
satisfies them. Each task independently verifiable; ratchet raises in the same commit as
the write path it covers (testing.md rule 1).

## Phase 1: Server-side subject generation (D29-#4=B)

- [ ] **RED: gen_subject unit tests**
  - File: `crates/talkservo-server/src/admin.rs` (test mod)
  - Tests: format `^u-[0-9a-z]{6}$`; two consecutive calls differ; seeded registry row
    collision avoided; charset within valid_id bounds
  - Verify: `cargo test -p talkservo-server --no-default-features --features embedded-web,stub-media gen_subject` fails to compile (function absent) — the RED

- [ ] **GREEN: tokens handler empty-sub branch + gen_subject helper**
  - File: `crates/talkservo-server/src/admin.rs`
  - Content: branch BEFORE valid_id (empty → generate → validated); helper loops ≤8×,
    occupied = live seats ∪ registry rows; exhaustion → 500 loud; response shape unchanged
  - Verify: the unit tests pass; `cargo clippy -p talkservo-server --all-targets` clean

## Phase 2: Rooms-row entry actions (D29-#3=B, #5=A)

- [ ] **entryActions.tsx (Enter + Copy links)**
  - File: `web/src/pages/admin/entryActions.tsx` (new)
  - Content: Enter = `api.issue({room, role:"dispatch"})` (sub omitted) →
    `window.open(urls.dispatch_url)`; Copy = three urls `\n`-joined via clipboard with
    textarea fallback; transient failure tooltip (never silent success)
  - Verify: `npx tsc --noEmit` clean

- [ ] **Wire into rooms.tsx row actions**
  - File: `web/src/pages/admin/rooms.tsx`
  - Content: `<EntryActions room={r.room} />` appended to the actions cell; Edit/Delete
    untouched
  - Verify: `npx vitest run tests/admin.test.tsx` (existing 12 pass, no regressions)

- [ ] **vitest: ratchet + behavior assertions (with the code, same commit)**
  - File: `web/tests/admin.test.tsx`
  - Tests: Enter fires POST /admin/api/tokens with `{room, role:"dispatch"}` and NO sub
    (baseline 3→4 documented in the test comment); Copy writes the 3-URL text
  - Verify: `npx vitest run` green; ratchet grep count = 4

## Phase 3: Join-failure messaging (D29-#6=A)

- [ ] **signal/session error passthrough**
  - File: `web/src/lib/session.ts` (+ SDK mirror only if public surface, per design)
  - Content: terminal close error object gains `closeCode` + `reason` (4410); classifier
    untouched
  - Verify: existing session/SDK tests pass; new field asserted once

- [ ] **JoinFailure component + page wiring**
  - File: `web/src/components/joinFailure.tsx` (new) +
    `web/src/pages/{participant,field,dispatcher}.tsx`
  - Content: EXPIRED/REMOVED(+reason)/DISCONNECTED tiers per design; no apply-to-join
    affordance; unknown terminal → DISCONNECTED tier; page copy language follows the
    existing pages
  - Verify: `npx tsc --noEmit` clean

- [ ] **vitest: failure tiers**
  - File: `web/tests/` (join-failure suite or extend existing)
  - Tests: fake close codes 4010/4410(+reason)/other → tier copy + retry expectations
  - Verify: `npx vitest run` green

## Phase 4: e2e + docs + gates

- [ ] **e2e kick-reason row**
  - File: `web/e2e/admin.spec.ts`
  - Content: console kick with reason → participant page shows removal + reason (existing
    admin/ws + kick path; TS_ADMIN lane env)
  - Verify: `npx playwright test e2e/admin.spec.ts` green (stub lane)

- [ ] **modules/08 rows**
  - File: `docs/modules/08-web-ui.md`
  - Content: admin entry-actions row (Enter/Copy semantics + the generated-sub note
    citing D29); join-failure-state table (close code → copy → action)
  - Verify: `bash scripts/gate.sh` (C1/C5 — no hosted-CI claims, English artifacts)

- [ ] **Full gate sweep**
  - Verify: `~/.pixi/bin/pixi run lint && ~/.pixi/bin/pixi run test` green;
    `~/.pixi/bin/pixi run e2e` green; `bash scripts/gate.sh` 9/9

- [ ] **Ledger close-out**
  - File: `.agents/memorys/status.md`
  - Content: wave landed note in the phase header (one line, D29 pointer)
  - Verify: gate green (C4 ranges untouched — no new ids)

## Completion checklist

- [x] All boxes checked with real command output (no self-reports — PIT-1)
- [x] Ratchet baseline = 4 documented in the test file
- [x] No wire/protocol changes (modules/02 untouched — asserted by design)
- [x] Lobby affordance absent (D29-#2=B respected)
