// Minimal fake WebAudio for unit tests — jsdom implements NO AudioContext at all
// (native-build... see docs/plans/test-transmit-track/design.md: DI seam + call-shape
// asserts only; asserting frequency/decode against our own mock is tautology, PIT-33
// family). Shape mirrors only what web/src/lib/testAudio.ts touches.
import { vi } from "vitest";

export function makeFakeTrack(label = "fake"): MediaStreamTrack {
  return { kind: "audio", label, readyState: "live", stop() {} } as unknown as MediaStreamTrack;
}

/** One fake AudioContext class; each instance tracks its oscillators/sources. */
export function makeFakeAudioContextCtor() {
  const contexts: FakeAudioContext[] = [];
  class FakeAudioContext {
    static get instances() {
      return contexts;
    }
    constructor() {
      contexts.push(this);
    }
    destination = { stream: null as MediaStream | null };
    state = "running";
    resume = vi.fn(async () => {
      this.state = "running";
    });
    close = vi.fn(async () => {
      this.state = "closed";
    });
    createdSources: FakeOscillator[] = [];
    createBufferSource(): AudioBufferSourceNode {
      const s = new FakeBufferSource(this);
      this.createdSources.push(s as unknown as FakeOscillator);
      return s as unknown as AudioBufferSourceNode;
    }
    decodeAudioData = vi.fn(async (_buf: ArrayBuffer) => ({ duration: 1.5 }));
    createOscillator(): OscillatorNode {
      const o = new FakeOscillator(this);
      this.createdSources.push(o);
      return o as unknown as OscillatorNode;
    }
    createMediaStreamSource(_t: MediaStream): MediaStreamAudioSourceNode {
      return { connect() {} } as unknown as MediaStreamAudioSourceNode;
    }
    createMediaStreamDestination(): MediaStreamAudioDestinationNode {
      const track = makeFakeTrack("fake-dest");
      this.destination = { stream: { getAudioTracks: () => [track] } as unknown as MediaStream };
      return {
        track,
        connect() {},
        stream: this.destination.stream,
      } as unknown as MediaStreamAudioDestinationNode;
    }
  }
  class FakeBufferSource {
    buffer: unknown = null;
    loop = false;
    started = false;
    stopped = false;
    constructor(public ctx: FakeAudioContext) {}
    connect() {}
    start() {
      this.started = true;
    }
    stop() {
      this.stopped = true;
    }
  }
  class FakeOscillator {
    frequency = { value: 0 };
    started = false;
    stopped = false;
    constructor(public ctx: FakeAudioContext) {}
    connect() {}
    start() {
      this.started = true;
    }
    stop() {
      this.stopped = true;
    }
  }
  return { FakeAudioContext, FakeOscillator };
}

export type FakeAudioContext = InstanceType<ReturnType<typeof makeFakeAudioContextCtor>["FakeAudioContext"]>;
