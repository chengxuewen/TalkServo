# Design: test-transmit-track

Read [proposal.md](proposal.md) first. This is the how — one client-side seam, zero
backend.

## Architecture

```
                    SETTINGS DRAWER (dev)
             ┌──────────────────────────────┐
             │ Test transmit (dev)  [toggle]│
             │ Source:  (•) 440Hz  ( ) hello│
             └──────────────┬───────────────┘
                            │ toggles
                            ▼
   ┌──────────── session.ts publish seam ─────────────┐
   │                                                  │
   │  toggle OFF (default):                           │
   │    getUserMedia(audio) ──▶ publishMic(micTrack)  │   ← today's path
   │                                                  │
   │  toggle ON  at join:                             │
   │    makeToneTrack() / makeClipTrack()             │
   │      └─ WebAudio ─▶ MediaStreamDestination       │
   │           └─▶ publishMic(synthTrack)             │
   │                                                  │
   │  toggle ON mid-session:                          │
   │    sender.replaceTrack(synthTrack)  (hot switch) │
   └──────────────────────┬───────────────────────────┘
                          ▼
        SAME floor path as a real mic (STAND-IN KEY):
   press → floor request → grant → D13 unpause → uplink
   release → D13 pause → (track stays alive, silent)
```

The stand-in key reuses the real PTT key's state machine — the ONLY delta is which track
the producer carries. This is deliberate: the sequence under test (request/grant/pause/
release) is the product behavior, not the audio source.

## Files to Touch

### Create

| File | Purpose |
|---|---|
| `web/src/lib/testAudio.ts` | `makeToneTrack(freq=440): MediaStreamTrack` + `makeClipTrack(url): Promise<MediaStreamTrack>` (OscillatorNode / decode→looping BufferSource → MediaStreamDestination), sharing ONE lazy module-level AudioContext (created on first factory call; the drawer click is the user gesture — gesture-resume is NEW code, there is no existing unlock convention: field.tsx:277's AudioCues is an honest stub that renders null; the only real precedent is the receive-side analyser pool at field.tsx:53-64, which creates-and-never-closes). Teardown = stop the active source node on toggle-off; the shared context lives for the page (matches the pool convention, avoids context-cap exhaustion from toggle cycling). ~50 lines. DI seam: factories accept an optional `{ ctx }` so unit tests inject a fake. |
| `web/assets/hello.ogg` | ~8 KB recorded clip, in-repo own recording (license-clean), loop-friendly overlap-added tail (no silence gaps — see E11 note in proposal). New directory: `web/assets/` does not exist today (Vite serves it via import). |
| `web/tests/testAudio.test.tsx` | Unit via DI: a minimal hand-rolled fake AudioContext/MediaStreamTrack (~30 lines, new test util — jsdom implements NO WebAudio/MediaStream; setup.ts mocks only matchMedia) asserting CALL SHAPE: context created once, oscillator started, `destination.stream.getAudioTracks()[0]` returned, source stopped on teardown. Real audibility is the live-lane e2e's job. |

### Modify

| File | Change |
|---|---|
| `packages/client/src/media.ts` | `ProducerLike` gains `replaceTrack(t): Promise<void>` (mediasoup-client Producer has it; the wrapper drops it today). `MediaManager` retains the producer handle at `publishMic` (today only the id is kept) and gains `setTestSource(track)` delegating to it. ~10 lines. |
| `packages/client/src/mediasoup-stack.ts` | `wrapSend` passes `replaceTrack` through from the real producer. ~5 lines. |
| `web/src/lib/session.ts` | Publish seam (~20 lines): read the test-transmit setting at join (drawer state is page-owned, passed in); choose synthetic vs mic track; expose `setTestTransmit(on: boolean, source)` → retained handle's `replaceTrack`. Keep the mic getUserMedia lazy — when the toggle is ON, do NOT open the mic at all (privacy + the no-mic-machine case where getUserMedia throws). |
| `web/src/pages/field.tsx` | Existing settings drawer: "Test transmit (dev)" toggle + source selector (sine/hello); wire to session's seam; label states it transmits to the room. |
| `web/src/pages/participant.tsx` | NEW small inline control (a compact select+switch next to the existing mic toggle — participant has no drawer today; a full drawer is unjustified surface for one control). |
| (dispatcher: explicitly OUT) | Dispatcher is a monitoring console with no PTT key to stand in for; its session shell keeps the seam but no UI. Documented in modules/08. |
| `web/e2e/media.spec.ts` (LIVE lane, extends the existing harness) | Test-transmit row: sender enables test transmit (sine) → `audioLevel()` (RTP bytesReceived delta — AnalyserNode RMS reads 0 in headless Chromium, measured helpers.ts:26) rises while granted, pre-grant ≈ 0, stops growing after release. Stub lane gets a publish-origin assert only (synthetic track label at the signaling surface). Live lane costs ~25 min per full run — the wave's calendar long pole, named here. |
| `docs/modules/08-web-ui.md` | Settings drawer: test-transmit row (behavior, sources, dev-only note, floor-respect statement). |

### Delete

None.

## Data Flow

**Cold (toggle ON before join)** — the no-microphone machine case:
```
join → drawer state says test:sine → makeToneTrack()
     → publishMic(synth) → floor request → grant → D13 unpause → room hears 440 Hz
     (getUserMedia NEVER called — no mic exists, none needed)
```

**Hot (toggle mid-session)**:
```
toggle ON  → setTestTransmit(true,'hello') → makeClipTrack() → sender.replaceTrack(synth)
toggle OFF → replaceTrack(mic) — mic getUserMedia happens HERE (first real mic use)
```

**Source switch while ON** (sine→hello or reverse) rides the same `replaceTrack` path —
one `setTestTransmit(true, otherSource)` call, no state churn.

**Release semantics** (shared): releasing the stand-in key pauses the producer (D13) —
the synthetic track keeps running locally (oscillator silent-upstream), which matches
real-mic behavior (track alive, upstream muted). No special-casing in the floor path.

## Integration Points

- **PTT key state machine**: the stand-in key CALLS the same handlers as the real key
  (press/release → floor request/release). Not a parallel implementation — a second
  trigger of the same one (the whole point: test the real steps).
- **AudioContext conventions**: one lazy module-level context for the test sources
  (precedent: the per-element receiver pool at field.tsx:53-64, which creates-and-never-
  closes); gesture-resume on the drawer click is NEW code (:277's AudioCues is a stub).
- **e2e lane**: fake-media-stream Chromium flags already in use; tone tier chosen for
  the live-lane e2e assert (`audioLevel` bytesReceived delta), clip tier for manual
  realism checks.
- **wire/server**: zero contact. modules/02 untouched. The only server-visible delta is
  "this producer carries a synthetic track" — indistinguishable at the RTP level and
  deliberately so.

## Rust Specifics

None. No crate changes.

## Error Handling

- `makeClipTrack` decode failure (corrupt/missing ogg): loud console error + drawer shows
  "test audio unavailable" state; toggle falls back to sine (never silent-success).
- AudioContext blocked (no user gesture yet): the toggle click is the gesture — unlock
  rides it; if the context still fails, error surfaces in the drawer (same loud path).
- `replaceTrack` rejection (rare): error propagates to the existing session error
  surface; toggle reverts to OFF state visually.

## Testing Strategy

1. **vitest unit** (`testAudio.test.tsx`, DI + fake WebAudio): call-shape asserts only
   (context created once / oscillator started / track from destination / teardown stops) —
   jsdom has no WebAudio, so frequency/decode "asserts" against a mock would be tautology
   (PIT-33 family); real decode+audibility is the live-lane e2e's job.
2. **vitest wiring**: toggling before join → publishMic receives synthetic track;
   getUserMedia not called (spy); mid-session toggle → replaceTrack called with synthetic.
3. **e2e, LIVE lane** (media.spec.ts harness): pre-grant RMS ≈ 0 → granted RMS rises →
   release stops growth (covers D13 both sides + the SC3 pre-grant edge). Stub lane:
   publish-origin assert only.
4. **Manual** (verification-before-completion): no-mic VM join with toggle ON (Success
   Criterion 1); hello tier heard by a second browser.
5. **Gates**: `npx vitest run && npx tsc --noEmit`; `pixi run test` (rust lanes untouched,
   must stay green); `bash scripts/gate.sh` 9/9.

## Dependencies

None new (WebAudio is platform; ogg asset is in-repo).
