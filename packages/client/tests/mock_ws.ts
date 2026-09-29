// Mock WebSocket + scriptable message server for SDK tests (no real network).

import type { ClientMessage, ServerMessage } from "../src/types.js";

type Ev = { open: []; message: [{ data: unknown }]; close: [] };

export const READY_OPEN = 1;
export const READY_CLOSED = 3;

export class MockWebSocket {
  private emitters: { [K in keyof Ev]: Array<(...args: Ev[K]) => void> } = {
    open: [],
    message: [],
    close: [],
  };
  on<K extends keyof Ev>(ev: K, fn: (...args: Ev[K]) => void): void {
    this.emitters[ev].push(fn);
  }
  private emit<K extends keyof Ev>(ev: K, ...args: Ev[K]): void {
    for (const fn of this.emitters[ev]) fn(...args);
  }
  static instances: MockWebSocket[] = [];
  readyState = 0; // CONNECTING until opened()
  url: string;
  sent: ClientMessage[] = [];
  // browser-style handler properties; emit() bridges to them
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
    this.on("open", () => this.onopen?.());
    this.on("message", (ev) => this.onmessage?.(ev));
    this.on("close", () => this.onclose?.());
  }

  /** Test drives connection establishment. */
  serverAccept() {
    this.readyState = READY_OPEN;
    this.emit("open");
  }

  serverPush(msg: ServerMessage) {
    this.emit("message", { data: JSON.stringify(msg) as unknown });
  }

  serverClose() {
    this.readyState = READY_CLOSED;
    this.emit("close");
  }

  send(data: string) {
    this.sent.push(JSON.parse(data) as ClientMessage);
  }

  close() {
    this.readyState = READY_CLOSED;
    this.emit("close");
  }

  /** All client messages of a given type. */
  sentOfType<T extends ClientMessage["type"]>(t: T) {
    return this.sent.filter((m) => m.type === t);
  }
}

export function lastSocket(): MockWebSocket {
  const s = MockWebSocket.instances.at(-1);
  if (!s) throw new Error("no socket created");
  return s;
}

export function resetSockets() {
  MockWebSocket.instances = [];
}

/** The standard J burst a peer receives after a successful join. */
export function joinBurst(selfId: string, role: "dispatch" | "field") {
  return [
    { type: "welcome", v: 1, peer_id: selfId, turn_creds: {} },
    {
      type: "router_caps",
      media_codecs: [{ mimeType: "audio/opus", channels: 1 }],
    },
    { type: "peer_list", peers: [{ id: selfId, role, connected_since_ms: 1 }], generation: 1 },
    role === "dispatch"
      ? {
          type: "server_snapshot",
          payload: {
            payload: "dispatch",
            mode: "hybrid",
            grants: [],
            muted: [],
            queue: [],
            peers: [{ id: selfId, role, connected_since_ms: 1 }],
            generation: 1,
          },
          generation: 1,
        }
      : {
          type: "server_snapshot",
          payload: {
            payload: "field",
            mode: "hybrid",
            grants: [],
            muted: [],
            peers: [{ id: selfId, role, connected_since_ms: 1 }],
            generation: 1,
          },
          generation: 1,
        },
  ] as ServerMessage[];
}
