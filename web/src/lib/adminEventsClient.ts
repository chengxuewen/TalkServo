// `/admin/ws` client for the console (plan-9 W2).
//
// Contract: docs/reference/admin-events.md. Kept free of React so the push paths
// are unit-testable with an injected socket, and deliberately dumb: it never
// invents state, it only forwards what the server said.
//
// The two cadences below exist because of a decision, not a default: a hint that
// arrives while a burst is in flight must not become one HTTP fetch per event
// (LOGS_DEBOUNCE_MS), and a reconnect must not become an accept-storm against the
// server's own connect budget (RECONNECT_MS is > 2s per tab, budget is 120/min —
// plan-9 R5).

import type { AdminEvent, AdminIn } from "./adminEvents.js";

/** Link state of THIS tab's socket. `down` is what makes the console say so
 *  instead of looking healthy while frozen (the failure mode wave-2 exists to kill). */
export type Link = "connecting" | "live" | "down";

export const RECONCILE_MS = 30_000;
/** fallback poll while there is no push: the reconcile stays the safety net, it
 *  just tightens until the socket comes back */
export const FALLBACK_MS = 3_000;
export const LOGS_DEBOUNCE_MS = 150;
const RECONNECT_MS = 2_500;
const PING_MS = 20_000;

/** Structural minimum (same shape the SDK uses), so tests can inject a fake. */
export interface Sock {
  send(data: string): void;
  close(): void;
  onopen: (() => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
  onclose: (() => void) | null;
  onerror: (() => void) | null;
}

export interface EventsHooks {
  /** read at connect time: a rotated/bounced session must not stay subscribed */
  token: () => string | null;
  onHint: () => void;
  onRooms: (rooms: unknown[]) => void;
  onLink: (state: Link) => void;
  wsFactory?: (url: string) => Sock;
}

export class AdminEvents {
  private readonly h: EventsHooks;
  private ws: Sock | null = null;
  private ping: ReturnType<typeof setInterval> | null = null;
  private retry: ReturnType<typeof setTimeout> | null = null;
  private stopped = true;

  constructor(h: EventsHooks) {
    this.h = h;
  }

  start(): void {
    if (!this.stopped) return;
    this.stopped = false;
    this.open();
  }

  stop(): void {
    this.stopped = true;
    // a stopped client has no link — reporting otherwise would leave the console
    // claiming it is live after a logout bounce (and lets a test drop the link
    // through the real code path instead of a test-only seam)
    this.h.onLink("down");
    if (this.ping) clearInterval(this.ping);
    if (this.retry) clearTimeout(this.retry);
    this.ping = this.retry = null;
    this.ws?.close();
    this.ws = null;
  }

  private open(): void {
    if (this.stopped) return;
    const token = this.h.token();
    if (!token) {
      // logged out: nothing to subscribe with, and nothing to retry either
      this.h.onLink("down");
      return;
    }
    this.h.onLink("connecting");
    const url = `${location.origin.replace(/^http/, "ws")}/admin/ws`;
    // typed as Sock at once: the browser socket and an injected fake share only this
    // structural shape, and a union here would lose the handler parameter types
    const ws: Sock = this.h.wsFactory
      ? this.h.wsFactory(url)
      : (new WebSocket(url) as unknown as Sock);
    this.ws = ws;

    const say = (frame: AdminIn) => {
      try {
        this.ws?.send(JSON.stringify(frame));
      } catch {
        /* a socket mid-teardown drops frames; onclose drives the retry */
      }
    };

    ws.onopen = () => {
      // credential first, then subscribe — never the token in the URL (access logs)
      say({ type: "auth", token });
      say({ type: "subscribe" });
      this.ping = setInterval(() => say({ type: "ping" }), PING_MS);
    };
    ws.onmessage = (ev) => {
      let msg: AdminEvent;
      try {
        msg = JSON.parse(String(ev.data)) as AdminEvent;
      } catch {
        return; // foreign frame: ignore rather than tear the link down
      }
      switch (msg.type) {
        case "hello":
          this.h.onLink("live");
          break;
        case "audit_hint":
          this.h.onHint();
          break;
        case "rooms_live":
          this.h.onRooms(msg.rooms);
          break;
        case "pong":
          break; // liveness only; the link is proven by frames arriving
      }
    };
    ws.onclose = () => {
      if (this.ping) clearInterval(this.ping);
      this.ping = null;
      this.ws = null;
      if (this.stopped) return;
      this.h.onLink("down");
      this.retry = setTimeout(() => this.open(), RECONNECT_MS);
    };
    ws.onerror = () => {
      /* the close event follows and owns the retry */
    };
  }
}
