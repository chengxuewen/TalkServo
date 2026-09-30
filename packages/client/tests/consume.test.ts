// Consume exchange (plan-3 gap #1): router caps captured, pull-model request
// pairing, ConsumeOk → handle opened.

import { describe, it, expect, beforeEach } from "vitest";
import { TalkServoClient, MediaManager, type MediaStack } from "../src/index.js";
import { MockWebSocket, lastSocket, resetSockets, joinBurst } from "./mock_ws.js";

beforeEach(() => resetSockets());

function connected() {
  const c = new TalkServoClient({
    role: "field",
    wsFactory: (u: string) => new MockWebSocket(u),
  });
  const p = c.connect("ws://mock", "jwt");
  const ws = lastSocket();
  ws.serverAccept();
  for (const m of joinBurst("listener", "field")) ws.serverPush(m);
  return { c, ws };
}

describe("consume exchange", () => {
  it("router_caps captured into the manager", async () => {
    const { c, ws } = connected();
    const mm = new MediaManager({ stack: {} as MediaStack });
    mm.attach(c);
    ws.serverPush({
      type: "router_caps",
      media_codecs: { codecs: [{ mimeType: "audio/opus" }], headerExtensions: [] },
    });
    expect(mm.routerCaps).toEqual({ codecs: [{ mimeType: "audio/opus" }], headerExtensions: [] });
  });

  it("consumeGranted sends Consume; ConsumeOk opens the handle", async () => {
    const { c, ws } = connected();
    const mm = new MediaManager({ stack: {} as MediaStack });
    mm.attach(c);

    const recv = {
      consume: async () => ({
        id: "c1",
        producerId: "prod-speaker",
        track: { enabled: true } as MediaStreamTrack,
        pause() {},
        resume() {},
        close() {},
      }),
    };
    mm.setRecvFactory(async () => recv);

    const opened: string[] = [];
    mm.on((e) => {
      if (e.kind === "audio-opened") opened.push(e.peerId);
    });

    // pull-model request
    mm.consumeGranted(["speaker"], new Map([["speaker", "prod-speaker"]]));
    const consumes = lastSocket().sentOfType("consume");
    expect(consumes).toEqual([{ type: "consume", producer_id: "prod-speaker" }]);

    // server answers
    ws.serverPush({
      type: "consume_ok",
      producer_id: "prod-speaker",
      rtp_parameters: { codecs: [] },
    });
    await new Promise((r) => setTimeout(r, 10));
    expect(opened).toEqual(["speaker"]);
    expect(mm.handleCount).toBe(1);
  });

  it("ConsumeOk for unknown producer_id is ignored", async () => {
    const { c, ws } = connected();
    const mm = new MediaManager({ stack: {} as MediaStack });
    mm.attach(c);
    ws.serverPush({ type: "consume_ok", producer_id: "ghost", rtp_parameters: {} });
    expect(mm.handleCount).toBe(0);
  });
});
