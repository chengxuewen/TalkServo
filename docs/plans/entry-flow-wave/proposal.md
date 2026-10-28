# Proposal: entry-flow-wave

**Date**: 2026-10-28
**Status**: DRAFT — pending user approval of this document
**Authority**: D29 (decisions.md, 2026-10-28) — entry-flow wave adjudicated one by one
(#1-#2 timing/scope, #3-#6 detail rulings); this document implements #3-#6 and D29's
companion items as one wave.

## What

Close the admin-to-room loop and make join failures legible — four user-visible changes,
zero wire/server-protocol changes except one issue-endpoint default:

1. **Server-side subject autogeneration** (D29-#4=B): `POST /admin/api/tokens` accepts an
   EMPTY `sub` and generates `u-` + 6 base36 chars (e.g. `u-a3f2k9`) at issue time, with a
   collision-retry loop. Fixed at issuance; travels with the link; D23 authority chain
   (audit actor/target, R7 terminal kick, S7.5 mute persistence) untouched.
2. **Rooms-row entry actions** (D29-#3=B): each Rooms registry row gains **Enter** (mint
   the admin's own dispatch token via the existing issue endpoint with a generated subject,
   then open `/dispatch/<room>?jwt=...`) and **Copy links** (D29-#5=A: all three join URLs
   as plain text, newline-joined, one click).
3. **Join-failure messaging** (D29-#6=A): a shared failure component on the three join
   pages (`/room`, `/field`, `/dispatch`) translating close codes the client already
   classifies: 4010 expired → "link expired, ask the operator for a new one" (no retry);
   4410 removed → "removed from the room" + the operator's reason (R7's words reach their
   target); other abnormal closes → keep auto-reconnect (existing behavior, unchanged).

## Why

Today's session hit every gap live: hand-typed subjects collided on `AlreadyJoined`
(`sub` = seat identity, ws.rs:237); the admin had to hop Rooms → Tokens → copy URL → open
browser to enter a room they just created; a participant with an expired link saw a bare
WebSocket error with no indication of what to do. Mainstream consoles (LiveKit dashboard,
BBB Greenlight, Zoom portal) all put Enter/Copy-link on the room row — the admin console
is the operator's cockpit and this wave makes it act like one.

## Scope

**In scope**
- `crates/talkservo-server/src/admin.rs`: tokens handler — empty-`sub` default branch
  + generation helper + collision check (occupied seats + registry)
- `packages/client` + `web`: subject passthrough stays as-is (generated value flows in
  the existing `sub` field — SDK/test mirrors unaffected)
- `web/src/pages/admin/rooms.tsx`: row actions (Enter, Copy links)
- `web/src/lib/session.ts` / `packages/client/src/signal.ts`: surface close code +
  reason on the error object (passthrough only; classification logic already exists)
- `web/src/pages/{participant,field,dispatcher}.tsx`: render the shared failure component
- Tests: Rust issue-path unit tests (generation format, collision, charset); vitest
  outbound-body assertions for the new write path (testing.md ratchet: adding a write
  path to adminApi usage must raise the baseline) + failure-tier copy assertions;
  e2e: one kick-row assertion (admin/ws infrastructure exists)
- Docs: modules/08 (admin entry actions row + join-failure-state table), D29 is already
  recorded; AGENTS.md ranges unchanged (no new ledger ids)

**Out of scope**
- Lobby / waiting room / apply-to-join (D29-#2=B, Beta-trigger; failure component must
  NOT grow an "apply" affordance)
- Sequential or role-prefixed subject formats (adjudicated against)
- Markdown-formatted link copy, per-link popover (adjudicated against)
- Any change to Tokens page's per-row copyable issue-result card
- Wire contract (modules/02) — no new SignalingMessage variants (plan-7 §4 ruling stands)

## Layers Affected

- HAL Core: no
- talkservo-server: **yes** (one handler branch + helper — no new endpoints, no wire)
- SFU host: no
- Web SPA: **yes** (primary surface)
- SDK packages/client: touch-free (sub is data, not shape) — verified by existing
  drift tests

## Transports Affected

- talkservo signaling: no new variants; issue endpoint response shape unchanged
  (generated sub rides the existing `sub` response field)
- admin REST/ws: no change

## Existing Specs

- `docs/modules/08-web-ui.md` — route table (C3 full-word paths), admin tab inventory;
  gains the entry-actions row and the failure-state table
- `docs/modules/06-deployment-security.md` — token/claims model (sub = PeerId;
  valid_id charset); the generation format must be recorded next to it or in D29 (already
  in D29)
- `.agents/memorys/decisions.md` D29 — the four rulings this wave implements (#3-#6)
- `.agents/rules/common/testing.md` — browser-lane ratchet (outbound-body assertions
  baseline 3; the Enter button's issue call must raise it)

## New Specs Needed

- None. The wave is UI + one handler default; modules/08 rows carry the design truth.

## Risks

- **Collision-check authority**: generated subs must avoid BOTH live seats and registry
  rows (a generated sub equal to a registered-but-offline subject would hijack that
  identity's re-entry). Mitigation: check the room actor's seat map via the rooms table
  AND `rooms.yaml` rows; retry ≤ 8 times then fail loud (6^36 space makes >1 retry
  astronomically unlikely; the bound is a guard, not a strategy).
- **Clipboard API in insecure contexts**: `navigator.clipboard` requires HTTPS or
  localhost. The current LAN flow is plain HTTP on a LAN IP. Mitigation: fall back to the
  existing `copy` helper's approach (textarea+execCommand) already proven in tokens.tsx.
- **4410 reason reachability**: the reason rides the close frame only at disconnect time;
  the join page must capture it BEFORE the auto-reconnect classifier drops the socket.
  Mitigation: signal.ts's existing 4410 branch already surfaces reason — passthrough only,
  verified by a unit test on the error object shape.
- **vitest ratchet drift**: adding the issue call from rooms.tsx is a new outbound-body
  path — forgetting the assertion is the exact PIT-28 shape. Mitigation: the ratchet's
  grep-based check rises with the wave (baseline 3 → 4), asserted in tasks.

## Success Criteria

1. `TS_ADMIN=1` issue with `{"room":"r1","role":"field"}` (no sub) returns a generated
   `u-xxxxxx` subject; the returned join links work (e2e join with the issued link).
2. Two consecutive empty-sub issues in the same room yield different subjects.
3. Rooms row Enter opens `/dispatch/<room>` connected as a generated dispatch subject
   (e2e or manual browser verification per verification-before-completion).
4. Copy links writes exactly three URLs (field/dispatch/participant) newline-joined —
   vitest asserts the generated text.
5. Kick with reason via admin console → participant page shows "removed" + reason, no
   retry loop (e2e row using the existing admin/ws + kick path).
6. Expired-token join shows the expired copy without retry (vitest, close-code fake).
7. `pixi run lint && pixi run test` green; e2e stub lane green; ratchet baseline raised.

## References

- D29 + its revision block (decisions.md, 2026-10-28) — rulings #1-#6 with sources
- ws.rs:237 (sub → PeerId), admin.rs:384-402 (tokens handler + valid_id), room.rs:543
  (AlreadyJoined), signal.ts R7 classification — the four code anchors of the problem
- Mainstream comparison recorded in D29 (PTT lineage = invite+persistent identity;
  LiveKit/BBB/Zoom row-level Enter/Copy precedents)
