// Media manager tests — plan-3 T2: AudioHandle lifecycle, micTruth,
// grant-release coupling, teardown, transport-failed.

import { describe, it, expect } from "vitest";
import { TalkServoClient, MediaManager, type MediaStack, type MediaEvent } from "../src/index.js";
import { MockWebSocket, lastSocket, resetSockets, joinBurst } from "./mock_ws.js";

function fakeTrack(): MediaStreamTrack {
  return { enabled: true, stop() {}, kind: "audio" } as unknown as MediaStreamTrack;
}

function makeStack(): MediaStack & { recv: never } {
  throw new Error("unused");
}
void makeStack;

function connectedClient(role: "dispatch" | "field" = "field", id = "p1") {
  const c = new TalkServoClient({
    role,
    wsFactory: (u: string) => new MockWebSocket(u),
  });
  const p = c.connect("ws://mock", "jwt");
  const ws = lastSocket();
  ws.serverAccept();
  for (const m of joinBurst(id, role)) ws.serverPush(m);
  ws.serverPush({ type: "floor_granted", grants: [], generation: 2 });
  return { c, ws };
}

describe("mic truth (D16 dual gate)", () => {
  it("track.enabled AND !serverPaused", () => {
    const { c } = connectedClient();
    const stack = {} as MediaStack;
    const mm = new MediaManager({ stack });
    mm.attach(c);

    const events: MediaEvent[] = [];
    mm.on((e) => events.push(e));

    void mm.publishMic(fakeTrack());
    expect(mm.micTruth()).toEqual({ trackEnabled: true, serverPaused: true });

    mm.setServerPaused(false); // grant path
    expect(mm.micTruth()).toEqual({ trackEnabled: true, serverPaused: false });
    expect(events.some((e) => e.kind === "mic-truth")).toBe(true);

    // track disabled locally → not speaking even if granted
    (mm as unknown as { micTrack: MediaStreamTrack | null }).micTrack!.enabled = false;
    mm.setServerPaused(false);
    expect(mm.micTruth().trackEnabled).toBe(false);
  });
});

describe("AudioHandle lifecycle (S-1/#2)", () => {
  it("release on revoke: peer leaving grants closes the consumer", async () => {
    const { c, ws } = connectedClient();
    const mm = new MediaManager({ stack: {} as MediaStack });
    mm.attach(c);

    const released: string[] = [];
    mm.on((e) => {
      if (e.kind === "audio-released") released.push(e.peerId);
    });

    // open a handle for a granted peer
    const recv = {
      consume: async () => ({
        id: "c1",
        producerId: "prod-1",
        track: fakeTrack(),
        pause() {},
        resume() {},
        close() {},
      }),
    };
    await mm.consume("speaker", "prod-1", recv, {});
    expect(mm.handleCount).toBe(1);

    // server revokes: grants drop the peer → handle released
    ws.serverPush({ type: "floor_idle", generation: 3, reason: null });
    expect(released).toEqual(["speaker"]);
    expect(mm.handleCount).toBe(0); // released handles removed from pool view
  });
});

describe("teardown paths", () => {
  it("media_restart tears down all handles (W-step 3 mirror)", async () => {
    const { c, ws } = connectedClient();
    const mm = new MediaManager({ stack: {} as MediaStack });
    mm.attach(c);
    const events: MediaEvent[] = [];
    mm.on((e) => events.push(e));

    const recv = {
      consume: async () => ({
        id: "c1", producerId: "p", track: fakeTrack(), pause() {}, resume() {}, close() {},
      }),
    };
    await mm.consume("spk", "p", recv, {});
    ws.serverPush({ type: "media_restart", room: "room-1", reason: "worker rebuild" });

    expect(mm.handleCount).toBe(0);
    expect(events.some((e) => e.kind === "media-teardown")).toBe(true);
  });

  it("transportFailed() triggers teardown + event (S-5b/#12)", () => {
    const { c } = connectedClient();
    const mm = new MediaManager({ stack: {} as MediaStack });
    mm.attach(c);
    const events: MediaEvent[] = [];
    mm.on((e) => events.push(e));
    mm.transportFailed();
    expect(events.some((e) => e.kind === "media-teardown")).toBe(true);
  });
});
