# Proposal: test-transmit-track

**Date**: 2026-10-28
**Status**: DRAFT — pending user approval of this document
**Authority**: D29 Revision 2 (decisions.md, 2026-10-28) — adjudicated #7-#9 one by one;
sibling of [entry-flow-wave](../entry-flow-wave/proposal.md) (the A' preset-phrase track is
NOT this document — it is a separate future wave; see "Out of scope").

## What

Give developer/testing environments a working mouth. A "Test transmit" toggle in the
settings drawer of the PTT pages acts as a STAND-IN PTT key: it replicates the
click-to-talk sequence verbatim (floor request → grant → uplink audio → release) with a
synthetic audio source instead of the microphone, so the full voice path can be exercised
on machines without audio input (dev VMs, CI containers) and in manual sessions where
speaking is impractical.

Two audio sources behind one toggle (user-selectable):
- **440 Hz sine** — code-generated, zero assets; live-lane e2e asserts via RTP bytesReceived
  delta.
- **recorded "hello" clip (ogg, looped)** — realism tier for manual checks (forward-
  insurance for the day D4/3A lands; see Risks).

Injection at the `publishMic` site (`web/src/lib/session.ts:229`): with the toggle ON at
join time, the synthetic track is published directly; toggling mid-session uses
`replaceTrack` for a hot switch. Zero backend change, zero wire change.

## Why

Adjudication #7 split the original "fake speak button" idea into two requirements with
different transports: the PRESET-PHRASE feature must bypass the voice path (hearing
argument — participants cannot attribute a sound on the shared stream), but the TEST
transmit must go THROUGH the voice path — the voice path itself is the thing being
tested. Today, machines without a microphone cannot exercise "click to talk" at all, and
the STUB e2e lane can only assert signaling state — remote-audio assertions (RTP
bytesReceived via `audioLevel`) exist only on the live lane (media.spec.ts, plan-4). This
track is the testing counterpart of the microphone: it must respect the floor machinery
(grant, D13 producer-pause, E11 max-hold) because those steps ARE the test.

## Scope

**In scope**
- `web/src/lib/session.ts`: source factory (`makeToneTrack()`, `makeClipTrack()`) +
  publish-site branch + `replaceTrack` hot-switch path
- `web/src/pages/field.tsx` (existing drawer) + `participant.tsx` (new inline control):
  "Test transmit (dev)" toggle with source selector. Dispatcher OUT — monitoring console,
  no PTT key to stand in for (per design)
- Asset: `web/assets/hello.ogg` (~8 KB, one-time recording, committed)
- Tests: vitest — toggle switches the published track away from getUserMedia origin;
  e2e (live lane) — tone mode: receiving-side audio level rises (`audioLevel()`
  bytesReceived delta)
- Docs: modules/08 settings-drawer section + a "test transmit (dev)" row

**Out of scope**
- **Preset phrases / quick reactions (A' track)** — separate future wave; its wire-variant
  design (and the plan-7 §4 revisiting it requires) is not part of this change
- One-click fixed-duration auto-play (rejected: fake timing skips the edges under test —
  max-hold E11, mid-hold kick)
- URL-driven auto-loop (`?tone=1` and descendants rejected: no floor semantics)
- TTS (`speechSynthesis` rejected: captureStream instability, per-browser divergence)
- Any server/sfu/wire change; any change visible to real participants outside the drawer

## Layers Affected

- HAL Core: no
- talkservo-server: no
- SFU host: no
- Web SPA: **yes** (only surface)
- SDK packages/client: **yes — small surface addition** (D14: unpublished, no compat
  cost): expose `replaceTrack` on `ProducerLike` (mediasoup-client has it; the wrapper
  strips it today) and retain the producer handle at the publish site. ~15 lines across
  `media.ts` + `mediasoup-stack.ts`. No wire change.

## Transports Affected

- None. No new signaling variants; normal floor/producer flow exactly as a human mic
  would take it.

## Existing Specs

- `docs/modules/08-web-ui.md` — settings drawer + PTT key state machine; gains the
  test-transmit row
- `web/src/pages/field.tsx:53-64` — the real WebAudio precedent (receive-side analyser
  pool, one context per element, created-and-never-closed). Note: :277's AudioCues is an
  honest stub that renders null — gesture-resume is NEW code in this change
- `.agents/memorys/decisions.md` D29 Revision 2 — rulings #7-#9 (split rationale, stand-in
  key, dual source + injection site; rejections recorded)
- `docs/modules/02` — wire contract: explicitly untouched (assertion, not assumption)

## New Specs Needed

- None. modules/08 row carries the design truth.

## Risks

- **3A chain eats the tone**: NOT a live risk today — no 3A ships in the pipeline (D4 is
  an adopted decision, never built; browsers apply their APM only to mic-capture tracks,
  and a WebAudio-destination track bypasses it). The hello-clip tier is forward-insurance
  for the day D4 lands; e2e asserts on the sine tier.
- **replaceTrack browser quirks**: target environment is Chromium (e2e lane reality);
  manual cross-browser testing is not in scope for a dev tool. Documented as dev-only.
- **Accidental enablement**: the toggle sits in the drawer with a "(dev)" label; a real
  participant could still flip it and transmit a loop to the room. Accepted: same trust
  level as mute/self-controls; floor machinery bounds the damage (they hold one seat).
- **Asset licensing**: the hello clip must be recorded in-repo (own voice, CC0) — no
  downloaded audio with unknown license.
- **E11 watchdog x clip gaps**: the max-hold watchdog downs silent holders; a looped clip
  with fade/silence gaps could read as inactivity during long holds. Mitigation: the sine
  tier is continuous (watchdog-safe) and is the e2e tier; the clip tier is manual-use, and
  its loop is built gap-free (overlap-added tail).

## Success Criteria

1. Toggle ON at join → room receives synthetic audio without any microphone present
   (manual pass on a machine with no audio input; the published track's empty label
   proves synthetic origin — a mic track would carry the device name).
2. Toggle mid-session → hot switch, remote hears the new source without reconnect.
3. Floor semantics preserved: pre-grant and post-release silence both asserted on the
   live lane (D13 pause is server-side RTP gating, verified track-agnostic:
   mediasoup_host.rs:303/359); grant arrives → audio flows.
4. Live-lane e2e (media.spec.ts, TS_LANE_LIVE=1): receiving-side audio level
   (`audioLevel` — RTP bytesReceived delta; AnalyserNode RMS reads 0 in headless
   Chromium, measured: helpers.ts:26) rises while the sender's test transmit is
   granted, and stops growing after release. Stub lane asserts publish-origin only.
5. `npx vitest run` + `npx tsc --noEmit` green; `pixi run test` rust lanes unaffected;
   modules/08 row lands; gate 9/9.

## References

- D29 Revision 2 (#7-#9) — the adjudications; the hearing-argument rejection of the
  audio-uplink PRESET feature (different requirement, same transport)
- webrtc-samples audio-helper (Google samples repo) — OscillatorNode → MediaStreamDestination
  → track, the reference pattern for `makeToneTrack`
- `web/src/lib/session.ts:229` — injection site; `field.tsx:53-64` — WebAudio conventions (the :277 AudioCues is a stub, see Existing Specs)
