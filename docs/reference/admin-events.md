# Admin Event Contract — `/admin/ws`

> Living reference (C2): this file mirrors a shipped surface and is updated in place.
> Source of truth: `crates/talkservo-server/src/admin_events.rs` (Rust enum) and
> `web/src/lib/adminEvents.ts` (client mirror). `web/tests/admin-events.test.ts`
> fails if the two drift — it compares the declared variant sets, which the
> signaling wire's replay fixture cannot cover (a replay only proves frames somebody
> actually sent).
> Design decision: [../plans/2026-10-21-alpha-wave2-realtime-control.md](../plans/2026-10-21-alpha-wave2-realtime-control.md).

## Why a second socket

`AdminClaims` deliberately carries **no `room` field**, and `/ws` decodes the
room-mandatory `Claims` — so an admin token structurally cannot open a room session,
even if both secrets were ever configured equal ([../modules/02-signaling-protocol.md](../modules/02-signaling-protocol.md)
§3). Putting console events on `/ws` would mean undoing that. On `/admin/ws` the
signaling contract stays frozen at its 27 variants (plan-7 §4) and the admin plane
versions itself.

## Handshake

```
console                                server
  │  GET /admin/ws (Upgrade)            │   budget + admin_secret checked BEFORE upgrade
  │ ──────────────────────────────────▶ │   401/503 as plain HTTP (no socket exists yet)
  │  101 Switching Protocols            │
  │ ── {"type":"auth","token":…} ─────▶ │   validate_admin(TS_ADMIN_SECRET)
  │ ◀──────────── {"type":"hello",…} ── │   or Close(1008)
  │ ── {"type":"ping"} ───────────────▶ │
  │ ◀──────────── {"type":"pong",…} ─── │
  │ ◀── {"type":"audit_hint"} / {"type":"rooms_live",…} ── │  (unsolicited, forever)
```

The credential is the **first frame**, never a header and never the query string: a
browser cannot set headers on a `WebSocket`, and a URL token would be written into
caddy's access log and the browser history — the same rule that keeps tokens out of
audit lines. After the first frame no further `auth` is read.

## Frames

### Server → console (`AdminEvent`, `tag = "type"`, snake_case)

| `type` | Fields | When |
|---|---|---|
| `hello` | `v`, `dropped`, `debounce_ms` | immediately after the credential is accepted. `dropped` = tabs this hub had to evict so far; `debounce_ms` = the server's live coalescing window, advertised so no client hard-codes a number the server can move |
| `audit_hint` | `v` | one line reached the audit ledger. **Carries no content** (see below) |
| `rooms_live` | `v`, `rooms[]` | the `/admin/api/rooms/live` body, pushed after activity settles. Same serializer as the REST poll (`admin::rooms_body`), so push and poll cannot disagree about shape |

### Console → server (`AdminIn`)

| `type` | Fields | Effect |
|---|---|---|
| `auth` | `token` | accepted once, first frame only; failure ⇒ Close(1008) |
| `subscribe` | — | accepted and ignored: both streams are always on. Present so a client need not guess whether to send it |
| `ping` | — | answered with `pong` (the WebSocket API gives a browser no control ping, so liveness must be an application frame) |

## Two rules this contract exists to keep

1. **A hint is not a row.** `audit_hint` carries no payload: the Logs table has exactly
   one path by which a line can appear, and it is a fetch of `/admin/api/audit`.
   Consequences: the socket can never disagree with the ledger, and `audit.rs` keeps
   stamping `seq` inside the writer task (no second sequence source, no race between
   sinks). Plan-9 R3=B; a payload-carrying hint would need a new adjudication.
2. **Push is a hint, the file is the ledger, the poll is the safety net.** The console
   reconciles on a slow timer regardless of socket health, and a dead socket is
   surfaced rather than looking like a quiet room (plan-9 R2=A).

## Coupling and limits

| Thing | Value / behavior | Where |
|---|---|---|
| Producer of every hint | `AuditTx::emit` — the audit write is queued **first**, then the hub is told; a console can never block, delay or fail an audited write | `crates/talkservo-server/src/audit.rs` |
| **Audit must be enabled** | no `audit_log` ⇒ no writer ⇒ no hints ⇒ no pushes at all. The console keeps working (it reconciles), and this is why the e2e harness has to set `TS_AUDIT_LOG` like the console deployment does | same |
| Hub bound | 64 frames per subscriber; a tab past that sees `Lagged`, hangs itself up and is counted — the publisher never blocks | `admin_events.rs` `HUB_CAPACITY` |
| Snapshot coalescing | 100 ms; one shared refresher task drains the burst, so N tabs cost **one** fan-out per burst, not N | `ROOMS_DEBOUNCE`, `spawn_rooms_refresher` |
| Connect budget | 120/min, a window of its own — deliberately **not** shared with the REST `admin_rate`, so reconnect backoff cannot lock the operator out of login or token issuance; over budget ⇒ 429 before upgrade | `ADMIN_WS_CONNECTS_PER_MIN`, `ws_rate_ok` |
| Auth deadline / frame cap | 10 s to present a credential, 4 KiB for that frame | `AUTH_DEADLINE`, `AUTH_FRAME_MAX_BYTES` |

## Versioning

`v` on every server frame comes from `ADMIN_EVENT_V` and is **independent of the
signaling `v: 1`**. Adding a variant here is additive for browsers (an SPA ignores an
unknown `type`); renaming or re-purposing one is breaking, and there is no second
consumer to migrate yet (the SDK never speaks this plane — D14 scope).

## Verification

* `crates/talkservo-server/tests/admin_events.rs` — auth-first enforcement, a **user**
  token refused on the admin socket (audience isolation, both directions), hint →
  live after a real mutation with the line present in the file, audit-off ⇒ socket
  works and stays silent, budget 429 at the constant, two tabs both hinted.
* `admin_events.rs` unit tests — no-subscriber is not a loss, lag is reported to the
  receiver, `absorb` collapses a burst, advertised debounce = compiled constant.
* `web/tests/admin-events.test.ts` — Rust/TS variant-set equality, tagged-snake_case
  shape, and the "no content in a hint" invariant.
