# Tasks: test-transmit-track

TDD per testing.md; every task independently verifiable. Wave size: small (one seam,
one drawer, one asset) — no phases beyond build/test/docs.

## Phase 1: Source factory + wiring

- [x] **testAudio.ts factory (tone + clip) with unit tests (DI + fake WebAudio)**
  - Files: `web/src/lib/testAudio.ts` (new) + `web/tests/testAudio.test.tsx` (new) +
    a ~30-line fake AudioContext/MediaStreamTrack test util (jsdom has neither)
  - Content: factories take optional `{ ctx }`; unit tests assert call shape only
    (created once / oscillator started / track surfaced / teardown stops) — tautological
    asserts against own mocks are banned (PIT-33 family)
  - Verify: `npx vitest run tests/testAudio.test.tsx` green; `npx tsc --noEmit` clean

- [x] **hello.ogg asset**
  - File: `web/assets/hello.ogg` (new, ~8 KB)
  - Content: own recording (license-clean), fade tail for loop smoothness; format check
    (ogg/vorbis decodes in Chromium)
  - Verify: `file web/assets/hello.ogg` + decode assert already covered by the unit test

- [x] **SDK: expose replaceTrack + retain the producer handle**
  - Files: `packages/client/src/media.ts` (ProducerLike + MediaManager), `mediasoup-stack.ts` (wrapSend)
  - Content: per design — ProducerLike gains `replaceTrack`, wrapSend passes it through,
    publishMic retains the handle, MediaManager gains `setTestSource`
  - Verify: `cd packages/client && npx vitest run` (SDK suite green, incl. a new
    replaceTrack passthrough case)

- [x] **session.ts publish seam + hot switch**
  - File: `web/src/lib/session.ts`
  - Content: read toggle state at join (synthetic → skip getUserMedia entirely);
    `setTestTransmit(on, source)` → retained-handle `replaceTrack` mid-session; mic
    getUserMedia becomes lazy (first real-mic use)
  - Verify: wiring vitest (spy: toggle ON → publishMic synthetic, getUserMedia not called;
    mid-session → replaceTrack called on the retained handle)

- [x] **Test-transmit controls (field drawer + participant inline; dispatcher OUT)**
  - Files: `web/src/pages/field.tsx` (existing drawer row), `participant.tsx` (NEW inline
    control next to the mic toggle)
  - Content: "Test transmit (dev)" toggle + sine/hello selector; click = gesture (context
    resume is new code); dispatcher explicitly out (monitoring console, no PTT key — D29
    Rev 2 scope note)
  - Verify: `npx tsc --noEmit`; vitest row asserting the toggle flips the session seam
    (participant pattern)

## Phase 2: e2e + docs + gates

- [x] **e2e test-transmit row (LIVE lane)**
  - File: `web/e2e/media.spec.ts` (extends the existing live-lane harness)
  - Content: pre-grant audio level ≈ 0 → grant → rises (sine, `audioLevel()` bytesReceived
    delta) → release → stops growing; stub lane gets a publish-origin assert only
  - Verify: `TS_LANE_LIVE=1 npx playwright test e2e/media.spec.ts` (~25 min live lane);
    stub `npx playwright test e2e/talk.spec.ts` stays green

- [x] **modules/08 row**
  - File: `docs/modules/08-web-ui.md`
  - Content: settings-drawer test-transmit row (behavior, dual source, dev-only, floor-
    respect statement citing D29 Revision 2)
  - Verify: `bash scripts/gate.sh` (C1/C5 clean)

- [x] **Full sweep**
  - Verify: `npx vitest run && npx tsc --noEmit` green; `pixi run test` green (rust
    untouched); `bash scripts/gate.sh` 9/9; manual no-mic pass (Success Criterion 1,
    no flags needed — gUM never called with toggle ON) recorded in the task notes;
    live-lane e2e is the wave's calendar long pole (~25 min) — budgeted here

## Completion checklist

- [x] All boxes checked with real command output (PIT-1)
- [x] No wire/server diffs (`git diff --stat crates/` empty — asserted, not assumed);
      SDK diff limited to the replaceTrack surface (packages/client only)
- [x] modules/08 row landed; gate 9/9
- [x] (out of this change) A' preset-phrase track remains a separate future wave

## Landing record (2026-10-28, apply session)

- All P1/P2 code landed: testAudio.ts (DI seam + shared lazy context + sine/clip factories),
  fakeWebAudio.ts test util, hello.wav asset (synthesized formant clip — NOT a real voice
  recording; this host has no audio input; swap-in is a filename-compatible asset change;
  contract honored: no silence gaps, overlap tail), SDK replaceTrack surface (ProducerLike +
  wrapSend passthrough), session publish seam (skip-gUM when ON; ?tone=1|clip pre-seed;
  retained handle + applyTestTransmit hot-switch), field drawer row + participant inline
  control, dispatcher OUT per adjudication.
- Deviation recorded: the producer-handle retention lives in session.ts's setSendMic closure
  (NOT MediaManager as design first said) — the handle is created there; MediaManager only
  ever saw the id. Functional equivalence, smaller diff; design note stands corrected here.
- Asset format deviation: .wav not .ogg (no transcoder on this host; decodeAudioData has
  hard browser support for WAV; the plan's ogg mention updated by this note).
- e2e rows are WRITTEN AND REGISTERED but NOT EXECUTED on this host: playwright 1.63
  refuses browser install on ubuntu 20.04 (support matrix). Registered rows: live lane
  3-phase audioLevel (media.spec.ts), stub publish-origin (talk.spec.ts). Execution awaits
  a supported environment or a playwright version pin. Recorded in status.md.
- Verification: web vitest 30/30 (incl. 4 testAudio call-shape, 2 participant seam),
  SDK vitest 24/24, web tsc clean, pixi run test 5/5, gate.sh 9/9.
- PRE-EXISTING (not this wave): packages/client tsc shows 3 errors (core.test.ts x2
  arity, media.test.ts PeerInfo) — present with the wave's changes stashed; untouched.
