// testAudio unit tests — CALL-SHAPE ONLY (design: test-transmit-track). jsdom has no
// WebAudio; asserting frequency/decode against our own fake is tautology (PIT-33 family).
// Real audibility/decode is the live-lane e2e's job (media.spec.ts, audioLevel).
import { afterEach, describe, expect, it, vi } from "vitest";
import { makeFakeAudioContextCtor } from "./fakeWebAudio.js";
import {
  makeToneTrack,
  makeClipTrack,
  stopTestSource,
  _setAudioContextCtorForTests,
} from "../src/lib/testAudio.js";

function inject() {
  const { FakeAudioContext } = makeFakeAudioContextCtor();
  _setAudioContextCtorForTests(FakeAudioContext as unknown as typeof AudioContext);
  return FakeAudioContext;
}

afterEach(() => {
  vi.unstubAllGlobals();
  stopTestSource();
});

describe("testAudio (call shape)", () => {
  it("makeToneTrack creates ONE context, starts one oscillator, returns the destination track", () => {
    const Ctx = inject();
    const track = makeToneTrack(440);
    expect(Ctx.instances).toHaveLength(1);
    const ctx = Ctx.instances[0];
    expect(ctx.createdSources).toHaveLength(1);
    expect(ctx.createdSources[0].started).toBe(true);
    expect(track).toBeTruthy();
  });

  it("second factory call REUSES the module-level context (no context churn)", () => {
    const Ctx = inject();
    makeToneTrack(440);
    makeToneTrack(660);
    expect(Ctx.instances).toHaveLength(1);
    expect(Ctx.instances[0].createdSources).toHaveLength(2);
  });

  it("stopTestSource stops the running source (teardown)", () => {
    const Ctx = inject();
    makeToneTrack(440);
    const src = Ctx.instances[0].createdSources[0];
    stopTestSource();
    expect(src.stopped).toBe(true);
  });

  it("makeClipTrack fetches + decodes + starts a looping source, resolves to a track", async () => {
    const Ctx = inject();
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: true,
        arrayBuffer: async () => new ArrayBuffer(8),
      })) as unknown as typeof fetch,
    );
    const track = await makeClipTrack("/assets/hello.ogg");
    expect(Ctx.instances[0].createdSources.length).toBe(1);
    expect(Ctx.instances[0].createdSources[0].started).toBe(true);
    expect(track).toBeTruthy();
  });
});
