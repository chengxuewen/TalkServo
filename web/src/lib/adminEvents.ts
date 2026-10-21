// Admin-plane contract mirror (plan-9 W1) — lives with its consumer, NOT in
// packages/client: the SDK is signaling-domain (D14 scope), while these frames
// belong to /admin/ws, which only the console speaks. Field names are snake_case
// exactly as the Rust serde tags; `tests/admin-events.test.ts` fails if the two
// files drift.

/** Server → console frames, discriminated by `type`. */
export type AdminEvent =
  /** first frame after the credential is accepted. `dropped` is the hub's count of
   *  tabs it had to evict for falling behind; `debounce_ms` is the server's live
   *  coalescing window, so the client never hard-codes a number the server can move. */
  | { type: "hello"; v: number; dropped: number; debounce_ms: number }
  /** "a line reached the ledger". Content is never pushed (plan-9 R3=B): the Logs
   *  table has exactly one path by which a row can appear, and it is a fetch. */
  | { type: "audit_hint"; v: number }
  /** the `/admin/api/rooms/live` body, pushed instead of polled. */
  | { type: "rooms_live"; v: number; rooms: unknown[] }
  /** answer to `ping`. A browser cannot send WS control pings, so liveness is an
   *  application frame or a dead socket is indistinguishable from a quiet room. */
  | { type: "pong"; v: number };

/** Console → server frames. `auth` must be the FIRST frame and only ever first: a
 *  browser cannot put a header on a WebSocket, and the token must not ride the URL
 *  either (access logs and history would keep a copy of a credential). */
export type AdminIn =
  | { type: "auth"; token: string }
  /** accepted and ignored — both streams are always on */
  | { type: "subscribe" }
  | { type: "ping" };

export const ADMIN_EVENT_V = 1;
/** Close code the server answers with for a missing/bad credential or an
 *  out-of-budget connect: a close code survives the browser WebSocket API, a
 *  response body does not. */
export const ADMIN_WS_POLICY_VIOLATION = 1008;
