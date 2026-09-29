// Signal transport: WS lifecycle, join handshake, heartbeat, reconnect
// backoff±jitter, TokenRefresh consumption, auth-expired terminal state.

import { PROTOCOL_VERSION, type ClientMessage, type ServerMessage } from "./types.js";

export interface SignalOptions {
  url: string;
  jwt: string;
  /** Heartbeat ping cadence (server sends nothing client-side; we detect
   *  dead sockets by expectingpongless lulls — PoC: plain ping frames). */
  heartbeatMs?: number;
  /** Reconnect backoff base (exponential, ±25% jitter). */
  backoffBaseMs?: number;
  /** Reconnect backoff ceiling. */
  backoffMaxMs?: number;
  /** DI seam for tests. */
  wsFactory?: (url: string) => LikeWebSocket;
}

/** Structural minimum the transport needs — lets tests inject mocks. */
export interface LikeWebSocket {
  readyState: number;
  send(data: string): void;
  close(): void;
  onopen: (() => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
  onclose: (() => void) | null;
  onerror: (() => void) | null;
}

export type SignalEvent =
  | { kind: "message"; msg: ServerMessage }
  | { kind: "open" }
  | { kind: "closed" }
  | { kind: "auth-expired"; detail: string }
  | { kind: "error"; code: string; detail: string };

const WS_OPEN = 1; // browser WebSocket readyState constant (node-safe)

const HEARTBEAT_MS_DEFAULT = 30_000;
const BACKOFF_BASE_MS_DEFAULT = 500;
const BACKOFF_MAX_MS_DEFAULT = 30_000;
const JOIN_TIMEOUT_MS = 10_000;

/** Browser default: wraps the global WebSocket into the structural shape. */
class NativeWebSocketAdapter implements LikeWebSocket {
  private ws: WebSocket;
  readyState: number;

  constructor(url: string) {
    this.ws = new WebSocket(url);
    this.readyState = this.ws.readyState;
    this.ws.addEventListener("open", () => {
      this.readyState = this.ws.readyState;
      this.onopen?.();
    });
    this.ws.addEventListener("message", (ev) => this.onmessage?.({ data: ev.data }));
    this.ws.addEventListener("close", () => {
      this.readyState = this.ws.readyState;
      this.onclose?.();
    });
    this.ws.addEventListener("error", () => this.onerror?.());
  }

  send(data: string): void {
    this.ws.send(data);
  }
  close(): void {
    this.ws.close();
  }
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
}

/** Raw WS lifecycle with the join handshake baked in. Emits typed events. */
export class SignalTransport {
  private ws: LikeWebSocket | null = null;
  private opts: Required<Pick<SignalOptions, "heartbeatMs" | "backoffBaseMs" | "backoffMaxMs">> &
    SignalOptions;
  private listeners = new Set<(e: SignalEvent) => void>();
  private attempt = 0;
  private closedByUs = false;
  private heartbeat: ReturnType<typeof setInterval> | null = null;
  private lastBeat = 0;
  private joinTimer: ReturnType<typeof setTimeout> | null = null;
  /** Latest token from TokenRefresh — consumed by reconnect (S-4/#3). */
  private currentJwt: string;

  constructor(opts: SignalOptions) {
    this.opts = {
      heartbeatMs: HEARTBEAT_MS_DEFAULT,
      backoffBaseMs: BACKOFF_BASE_MS_DEFAULT,
      backoffMaxMs: BACKOFF_MAX_MS_DEFAULT,
      ...opts,
    };
    this.currentJwt = opts.jwt;
  }

  on(fn: (e: SignalEvent) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private emit(e: SignalEvent) {
    for (const fn of this.listeners) fn(e);
  }

  /** Open the socket and run the join handshake. Resolves after Welcome. */
  connect(): Promise<void> {
    this.closedByUs = false;
    return new Promise((resolve, reject) => {
      const url = this.opts.url;
      const ws: LikeWebSocket = this.opts.wsFactory
        ? this.opts.wsFactory(url)
        : new NativeWebSocketAdapter(url);
      this.ws = ws;

      const joinGuard = setTimeout(() => {
        reject(new Error("join timeout (10s)"));
        ws.close();
      }, JOIN_TIMEOUT_MS);
      this.joinTimer = joinGuard;

      ws.onopen = () => {
        this.send({ type: "join", v: PROTOCOL_VERSION, jwt: this.currentJwt });
      };

      ws.onmessage = (ev) => {
        let msg: ServerMessage;
        try {
          msg = JSON.parse(String(ev.data)) as ServerMessage;
        } catch {
          return; // non-JSON frame — ignore
        }
        switch (msg.type) {
          case "welcome": {
            clearTimeout(joinGuard);
            this.attempt = 0;
            this.startHeartbeat();
            this.emit({ kind: "open" });
            resolve();
            this.emit({ kind: "message", msg }); // facade reduces it (selfId)
            return;
          }
          case "error": {
            if (msg.code === "already_joined") {
              clearTimeout(joinGuard);
              reject(new Error(`already_joined: ${msg.detail}`));
              return;
            }
            if (msg.code === "expired_token" || msg.code === "invalid_token") {
              clearTimeout(joinGuard);
              this.closedByUs = true; // terminal: no auto-retry (S-4/#3)
              this.emit({ kind: "auth-expired", detail: msg.detail });
              reject(new Error(`auth-expired: ${msg.code}`));
              return;
            }
            this.emit({ kind: "error", code: msg.code, detail: msg.detail });
            return;
          }
          default:
            this.emit({ kind: "message", msg });
        }
      };

      ws.onclose = () => {
        this.stopHeartbeat();
        clearTimeout(joinGuard);
        this.emit({ kind: "closed" });
        if (!this.closedByUs) this.scheduleReconnect();
      };

      ws.onerror = () => {
        // close event follows; reconnect logic lives there
      };
    });
  }

  send(msg: ClientMessage): void {
    if (this.ws && this.ws.readyState === WS_OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  /** Consume a fresh token (TokenRefresh push) for the next reconnect. */
  updateToken(jwt: string): void {
    this.currentJwt = jwt;
  }

  close(): void {
    this.closedByUs = true;
    this.stopHeartbeat();
    this.ws?.close();
    this.ws = null;
  }

  get connected(): boolean {
    return this.ws?.readyState === WS_OPEN;
  }

  private startHeartbeat() {
    this.stopHeartbeat();
    // Browsers cannot emit WS ping frames from JS; the server's heartbeat
    // sweep (E1) closes dead halves server-side, and a dead client socket
    // surfaces via onclose. This timer only marks an alive-ness flag.
    this.heartbeat = setInterval(() => {
      this.lastBeat = Date.now();
    }, this.opts.heartbeatMs);
  }

  private stopHeartbeat() {
    if (this.heartbeat) clearInterval(this.heartbeat);
    this.heartbeat = null;
  }

  private scheduleReconnect() {
    const base = this.opts.backoffBaseMs;
    const max = this.opts.backoffMaxMs;
    const exp = Math.min(max, base * 2 ** this.attempt);
    const jitter = exp * 0.25 * (Math.random() * 2 - 1); // ±25%
    const delay = Math.max(100, exp + jitter);
    this.attempt += 1;
    setTimeout(() => {
      if (!this.closedByUs) {
        this.connect().catch(() => {
          /* onclose drives the next attempt */
        });
      }
    }, delay);
  }
}
