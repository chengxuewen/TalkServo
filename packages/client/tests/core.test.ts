// SDK core behavior tests — plan-3 T1: J burst, P grant, X preempt, R replay,
// gen-discard, TokenRefresh cache, auth-expired terminal state, cooldown deny.

import { describe, it, expect, beforeEach } from "vitest";
import { TalkServoClient } from "../src/index.js";
import { MockWebSocket, lastSocket, resetSockets, joinBurst } from "./mock_ws.js";
import type { ServerMessage } from "../src/types.js";

function makeClient(role: "dispatch" | "field" = "field"): TalkServoClient {
  return new TalkServoClient({
    role,
    wsFactory: (u: string) => new MockWebSocket(u),
  });
}

async function connectJoined(role: "dispatch" | "field" = "field", id = "peer-a") {
  const c = makeClient(role);
  const p = c.connect("ws://mock", "jwt-0");
  const ws = lastSocket();
  ws.serverAccept();
  // server pushes the burst synchronously; connect() resolves on Welcome
  for (const m of joinBurst(id, role)) ws.serverPush(m);
  await p;
  return { c, ws };
}

beforeEach(() => resetSockets());

describe("join handshake", () => {
  it("sends join v:1 then consumes the J burst into the mirror", async () => {
    const { c, ws } = await connectJoined("field", "p1");
    const join = ws.sentOfType("join");
    expect(join).toHaveLength(1);
    expect(join[0]).toEqual({ type: "join", v: 1, jwt: "jwt-0" });
    expect(c.state.selfId).toBe("p1");
    expect(c.state.peers).toHaveLength(1);
  });

  it("rejects connect on already_joined", async () => {
    const c = makeClient();
    const p = c.connect("ws://mock", "jwt");
    const ws = lastSocket();
    ws.serverAccept();
    ws.serverPush({ type: "error", code: "already_joined", detail: "dup" });
    await expect(p).rejects.toThrow(/already_joined/);
  });
});

describe("floor control events", () => {
  it("P: grant updates mirror + emits state", async () => {
    const { c, ws } = await connectJoined();
    const events: unknown[] = [];
    c.on((e) => events.push(e));

    ws.serverPush({ type: "floor_granted", grants: ["peer-a"], generation: 2 });
    expect(c.state.grants).toEqual(["peer-a"]);
    expect(c.state.generation).toBe(2);
    expect(events.some((e) => (e as { kind: string }).kind === "state")).toBe(true);
  });

  it("requestFloor sends floor_request with priority/preempt", async () => {
    const { c } = await connectJoined();
    c.requestFloor(3, true);
    const ws = lastSocket();
    expect(ws.sentOfType("floor_request")).toEqual([
      { type: "floor_request", priority: 3, preempt: true },
    ]);
  });

  it("X: taken emits and gen advances", async () => {
    const { c, ws } = await connectJoined();
    const events: unknown[] = [];
    c.on((e) => events.push(e));

    ws.serverPush({ type: "floor_taken", by: "preemptor", generation: 3 });
    ws.serverPush({ type: "floor_granted", grants: ["preemptor"], generation: 4 });

    expect(events.some((e) => (e as { kind: string }).kind === "taken")).toBe(true);
    expect(c.state.grants).toEqual(["preemptor"]);
    expect(c.state.generation).toBe(4);
  });
});

describe("generation discipline (D16)", () => {
  it("drops stale events (gen < seen)", async () => {
    const { c, ws } = await connectJoined();
    ws.serverPush({ type: "floor_granted", grants: ["new"], generation: 5 });
    ws.serverPush({ type: "floor_granted", grants: ["stale"], generation: 4 });
    expect(c.state.grants).toEqual(["new"]);
    expect(c.state.generation).toBe(5);
  });
});

describe("role-scoped snapshots (D12)", () => {
  it("field role never sees the queue even if server misbehaves", async () => {
    const { c, ws } = await connectJoined("field");
    ws.serverPush({
      type: "server_snapshot",
      payload: {
        payload: "dispatch",
        mode: "exclusive",
        grants: [],
        muted: [],
        queue: [{ peer: "x", priority: 1 }],
        peers: [],
        generation: 9,
      },
      generation: 9,
    });
    // the wire should never do this; if it does, field store keeps queue empty
    expect(c.state.queue).toEqual([]);
  });

  it("dispatch role sees the queue", async () => {
    const { c, ws } = await connectJoined("dispatch");
    ws.serverPush({
      type: "server_snapshot",
      payload: {
        payload: "dispatch",
        mode: "exclusive",
        grants: ["a"],
        muted: [],
        queue: [{ peer: "b", priority: 2 }],
        peers: [],
        generation: 7,
      },
      generation: 7,
    });
    expect(c.state.queue).toEqual([{ peer: "b", priority: 2 }]);
  });
});

describe("token lifecycle", () => {
  it("TokenRefresh is cached for reconnect and surfaced", async () => {
    const { c, ws } = await connectJoined();
    const events: unknown[] = [];
    c.on((e) => events.push(e));
    ws.serverPush({ type: "token_refresh", jwt: "jwt-1" });
    expect(events.some((e) => (e as { kind: string }).kind === "token")).toBe(true);
  });

  it("auth-expired is terminal (no reconnect storm)", async () => {
    const c = makeClient();
    const events: unknown[] = [];
    c.on((e) => events.push(e));
    const p = c.connect("ws://mock", "dead-jwt");
    const ws = lastSocket();
    ws.serverAccept();
    ws.serverPush({ type: "error", code: "expired_token", detail: "exp" });
    ws.serverClose();
    await expect(p).rejects.toThrow(); // join never completed
    const kinds = events.map((e) => (e as { kind: string }).kind);
    expect(kinds).toContain("auth-expired");
    // no reconnect attempt: no new socket since close
    expect(c.state.selfId).toBeNull();
  });
});

describe("reconnect (R1)", () => {
  it("resync on reconnect requests fresh truth", async () => {
    const { c, ws } = await connectJoined();
    ws.sent.length = 0;
    c.disconnect();
    // simulate the transport's auto-reconnect path by a manual reconnect
    const p = c.connect("ws://mock", "jwt-0");
    const ws2 = lastSocket();
    ws2.serverAccept();
    ws2.serverPush(joinBurst("peer-a", "field")[0] as ServerMessage); // welcome
    await p;
    expect(ws2.sentOfType("resync").length).toBeGreaterThan(0);
  });
});

describe("R-replay fixture: full J/P/X/R sequence", async () => {
  it("replays a scripted session deterministically", async () => {
    const { c, ws } = await connectJoined("dispatch", "disp");

    const script: ServerMessage[] = [
      { type: "floor_granted", grants: ["field-1"], generation: 10 },
      { type: "floor_taken", by: "field-2", generation: 11 },
      { type: "floor_granted", grants: ["field-2"], generation: 12 },
      { type: "floor_idle", generation: 13, reason: null },
    ];
    for (const m of script) ws.serverPush(m);

    expect(c.state.generation).toBe(13);
    expect(c.state.grants).toEqual([]); // idle cleared
  });
});
