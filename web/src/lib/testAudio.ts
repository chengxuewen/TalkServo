// testAudio.ts — synthetic audio sources for the "Test transmit (dev)" stand-in PTT key
// (docs/plans/test-transmit-track; D29 Revision 2 #7-#9).
//
// Design contract:
//   * ONE lazy module-level AudioContext + ONE MediaStreamDestination, shared by both
//     factories — created on first use, lives for the page (matches the field.tsx
//     receiver-pool convention of create-and-never-close; a per-toggle context would hit
//     the browser's context cap after repeated cycling). Teardown = stop the active source
//     node, never close the context; the destination track identity is stable so
//     replaceTrack round-trips are cheap.
//   * Gesture note: the drawer toggle click is the user gesture — call resumeIfNeeded()
//     from it (the context may start suspended without one). This is NEW code: field's
//     AudioCues is a stub; there was no unlock convention to reuse.
//   * DI seam: `_setAudioContextCtorForTests` lets unit tests inject a fake (jsdom has no
//     WebAudio); production passes nothing.
//   * Sources: 440 Hz sine (code, continuous — watchdog-safe for E11) and a looped ogg
//     clip (realism tier; asset contract = overlap-added tail, no silence gaps).
let Ctor: typeof AudioContext | null = null;
let ctx: AudioContext | null = null;
let dest: MediaStreamAudioDestinationNode | null = null;
let activeSource: AudioScheduledSourceNode | null = null;

/** Test-only DI: swap the AudioContext constructor (jsdom has none). */
export function _setAudioContextCtorForTests(c: typeof AudioContext): void {
  Ctor = c;
  ctx = null;
  dest = null;
  activeSource = null;
}

function audioCtx(): AudioContext {
  if (!ctx) {
    Ctor ??= window.AudioContext;
    if (!Ctor) throw new Error("AudioContext unavailable (no WebAudio in this environment)");
    ctx = new Ctor();
  }
  if (ctx.state === "suspended") void ctx.resume();
  return ctx;
}

function destination(): MediaStreamAudioDestinationNode {
  dest ??= audioCtx().createMediaStreamDestination();
  return dest;
}

function swapSource(next: AudioScheduledSourceNode | null): void {
  if (activeSource) {
    try {
      activeSource.stop();
    } catch {
      // already stopped — idempotent teardown
    }
  }
  activeSource = next;
}

function activate(next: AudioScheduledSourceNode): MediaStreamTrack {
  swapSource(next);
  next.start();
  const track = destination().stream.getAudioTracks()[0];
  if (!track) throw new Error("MediaStreamDestination produced no audio track");
  return track;
}

/** Call from the toggle's click handler — the user gesture unlocks the context. */
export function resumeIfNeeded(): void {
  const c = audioCtx();
  if (c.state === "suspended") void c.resume();
}

/** Continuous sine (watchdog-safe: no silence gaps for E11 to read as inactivity). */
export function makeToneTrack(freq = 440): MediaStreamTrack {
  const osc = audioCtx().createOscillator();
  osc.frequency.value = freq;
  osc.connect(destination());
  return activate(osc);
}

/** Looped ogg clip (manual realism tier). Fetches + decodes; asset has no silence gaps. */
export async function makeClipTrack(url: string): Promise<MediaStreamTrack> {
  const c = audioCtx();
  const buf = await (await fetch(url)).arrayBuffer();
  const audio = await c.decodeAudioData(buf);
  const src = c.createBufferSource();
  src.buffer = audio;
  src.loop = true;
  src.connect(destination());
  return activate(src);
}

/** Teardown: stop the active source (context + destination stay alive for the page). */
export function stopTestSource(): void {
  swapSource(null);
}
