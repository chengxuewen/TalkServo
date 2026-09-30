// TalkServoClient — SDK facade (D14): wire/media/state owned here; views stay thin.

import { SignalTransport, type LikeWebSocket, type SignalEvent } from "./signal.js";
import { initialMirror, reduce, type FloorMirror } from "./store.js";
import {
  PROTOCOL_VERSION,
  type ClientMessage,
  type FloorMode,
  type Role,
  type ServerMessage,
} from "./types.js";

export interface ClientOptions {
  role: Role;
  url?: string;
  /** DI seam: custom WebSocket constructor (tests / workers). */
  wsFactory?: (url: string) => LikeWebSocket;
}

export type ClientEvent =
  | { kind: "connected"; selfId: string }
  | { kind: "state"; mirror: FloorMirror }
  | { kind: "denied"; reason: string }
  | { kind: "taken"; by: string }
  | { kind: "media-failed"; peer: string }
  | { kind: "media-restart"; room: string; reason: string }
  | { kind: "token"; jwt: string }
  | { kind: "auth-expired"; detail: string }
  | { kind: "disconnected" }
  | { kind: "error"; code: string; detail: string };

export class TalkServoClient {
  readonly role: Role;
  private signal: SignalTransport | null = null;
  private mirror: FloorMirror = initialMirror();
  private listeners = new Set<(e: ClientEvent) => void>();
  private opts: ClientOptions;

  constructor(opts: ClientOptions) {
    this.opts = opts;
    this.role = opts.role;
  }

  on(fn: (e: ClientEvent) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  /** Raw wire tap (media manager internal use: consume exchange pairing). */
  onMessage(fn: (msg: ServerMessage) => void): () => void {
    this.rawListeners.add(fn);
    return () => this.rawListeners.delete(fn);
  }
  private rawListeners = new Set<(msg: ServerMessage) => void>();

  private emit(e: ClientEvent) {
    for (const fn of this.listeners) fn(e);
  }

  get state(): FloorMirror {
    return this.mirror;
  }

  /** Connect + join. Resolves once Welcome lands (selfId known). */
  async connect(url: string, jwt: string): Promise<void> {
    const signal = new SignalTransport({
      url,
      jwt,
      // DI seam passthrough (tests inject MockWebSocket here)
      ...(this.opts.wsFactory ? { wsFactory: this.opts.wsFactory } : {}),
    });
    this.signal = signal;

    signal.on((e: SignalEvent) => this.onSignal(e));

    await signal.connect();
  }

  requestFloor(priority = 0, preempt = false): void {
    this.send({ type: "floor_request", priority, preempt });
  }

  releaseFloor(): void {
    this.send({ type: "floor_release" });
  }

  changeMode(mode: FloorMode): void {
    this.send({ type: "mode_change", mode });
  }

  mutePeer(peer: string, on: boolean): void {
    this.send({ type: "mute_set", peer, on });
  }

  /** R1: ask for a fresh snapshot (reconnect recovery, stale-view repair). */
  resync(): void {
    this.send({ type: "resync" });
  }

  disconnect(): void {
    this.signal?.close();
    this.signal = null;
    this.emit({ kind: "disconnected" });
  }

  /** Media-plane passthrough (media manager, T2) — typed hole, filled there. */
  sendRaw(msg: ClientMessage): void {
    this.send(msg);
  }

  get protocolVersion(): number {
    return PROTOCOL_VERSION;
  }

  private send(msg: ClientMessage) {
    this.signal?.send(msg);
  }

  private onSignal(e: SignalEvent) {
    switch (e.kind) {
      case "message":
        this.handleMessage(e.msg);
        return;
      case "open":
        // mirror keeps selfId across reconnects; request fresh truth (R1)
        this.send({ type: "resync" });
        return;
      case "closed":
        this.emit({ kind: "disconnected" });
        return;
      case "auth-expired":
        this.emit({ kind: "auth-expired", detail: e.detail });
        return;
      case "error":
        this.emit({ kind: "error", code: e.code, detail: e.detail });
        return;
    }
  }

  private handleMessage(msg: ServerMessage) {
    for (const fn of this.rawListeners) fn(msg);
    // TokenRefresh: cache for reconnect + surface (S-4)
    if (msg.type === "token_refresh") {
      this.signal?.updateToken(msg.jwt);
      this.emit({ kind: "token", jwt: msg.jwt });
      return;
    }

    const next = reduce(this.mirror, msg, this.role);
    if (next !== null && next !== this.mirror) {
      this.mirror = next;
      this.emit({ kind: "state", mirror: next });
    }

    switch (msg.type) {
      case "welcome":
        this.emit({ kind: "connected", selfId: msg.peer_id });
        // first snapshot request after welcome (server already sends one, but
        // an explicit resync is harmless and repairs late-join races)
        return;
      case "floor_denied":
        this.emit({ kind: "denied", reason: msg.reason });
        return;
      case "floor_taken":
        this.emit({ kind: "taken", by: msg.by });
        return;
      case "media_failed":
        this.emit({ kind: "media-failed", peer: msg.peer });
        return;
      case "media_restart":
        this.emit({ kind: "media-restart", room: msg.room, reason: msg.reason });
        return;
      default:
        return;
    }
  }
}

export * from "./types.js";
export * from "./media.js";
export type { LikeWebSocket, SignalOptions } from "./signal.js";
export { initialMirror, type FloorMirror } from "./store.js";
