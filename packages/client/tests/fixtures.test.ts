// Fixture replay (cross F3 closure): the canonical transcript RECORDED by
// the server (crates/talkservo-server/tests/record_fixtures.rs) is replayed
// into a client — one artifact, both ends, drift-proof.

import { describe, it, expect, beforeEach } from "vitest";
import { readFileSync } from "node:fs";
import { TalkServoClient } from "../src/index.js";
import { MockWebSocket, lastSocket, resetSockets } from "./mock_ws.js";

interface Frame {
  from: string;
  msg: { type: string; [k: string]: unknown };
}

const TRANSCRIPT = JSON.parse(
  readFileSync(
    new URL("../../../docs/reference/fixtures/ws-transcript.json", import.meta.url),
    "utf8",
  ),
) as { protocol_version: number; frames: Frame[] };

beforeEach(() => resetSockets());

describe("canonical transcript replay", () => {
  it("protocol_version matches the SDK constant", () => {
    expect(TRANSCRIPT.protocol_version).toBe(1);
    expect(new TalkServoClient({ role: "field" }).protocolVersion).toBe(
      TRANSCRIPT.protocol_version,
    );
  });

  it("dispatch client reduces the full J/P/X/R transcript to a consistent end state", async () => {
    const c = new TalkServoClient({
      role: "dispatch",
      wsFactory: (u: string) => new MockWebSocket(u),
    });
    const p = c.connect("ws://mock", "jwt");
    const ws = lastSocket();
    ws.serverAccept();

    // feed ONLY frames the dispatcher would receive (role-scoped view)
    for (const frame of TRANSCRIPT.frames) {
      if (frame.from !== "dispatcher") continue;
      ws.serverPush(frame.msg as never);
    }
    await p;

    // end state after P/X/R: chief preempted and then released → idle
    expect(c.state.selfId).toBe("chief");
    expect(c.state.grants).toEqual([]);
    expect(c.state.mode).toBe("exclusive");
    expect(c.state.peers.map((p) => p.id).sort()).toEqual(["alpha", "chief"]);
  });

  it("field client ignores dispatch-only frames and still converges", async () => {
    const c = new TalkServoClient({
      role: "field",
      wsFactory: (u: string) => new MockWebSocket(u),
    });
    const p = c.connect("ws://mock", "jwt");
    const ws = lastSocket();
    ws.serverAccept();

    // field replay gets EVERYTHING raw (server never sends dispatch-scoped
    // data to field, but the store must stay sane even if frames interleave)
    for (const frame of TRANSCRIPT.frames) {
      if (frame.from !== "alpha") continue;
      ws.serverPush(frame.msg as never);
    }
    await p;

    expect(c.state.selfId).toBe("alpha");
    expect(c.state.queue).toEqual([]); // field never sees a queue (D12)
  });
});
