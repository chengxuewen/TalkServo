# Plan-9: Alpha wave-2 — Realtime Control Plane (D23 deferred family)

> **STATUS: W1 + W3 LANDED 2026-10-21** (`/admin/ws` push plane with its own `AdminEvent` contract, bounded hub + counted eviction, own connect budget, `docs/reference/admin-events.md`, TS mirror + drift gate, 6 integration + 7 unit tests, media.ts hygiene). **W2 (console adoption) and the §4 latency measurement remain — and R7 (§10: a kick loses to the SDK's auto-reconnect) needs a ruling before anyone touches the e2e row.** Rulings R1-R5 in §9; R6 dissolved by R3.
> Design truth: [../modules/08-web-ui.md](../modules/08-web-ui.md) (console surface), [../modules/02-signaling-protocol.md](../modules/02-signaling-protocol.md) §3 (claims shape, audience isolation), [../modules/01-floor-model.md](../modules/01-floor-model.md) (D13 gating), [../modules/06-deployment-security.md](../modules/06-deployment-security.md) (auth ladder).
> Ledger: D18 (admin minimal surface), D23 + its UI annex ("operator-audible Listen = wave-2 … deferred WITH AdminEvent WS push as the wave-2 protocol batch"), acceptance #12 in [../architecture.md](../architecture.md) §4.

## 0. Why this wave exists

Two things were deferred **as a family** by D23, and they share one root: the control plane has no push channel.

1. The admin console is a **poller**. Monitor re-fetches `/admin/api/rooms/live` every 3 s (`web/src/pages/admin/monitor.tsx:28`), Logs is manual-refresh by design ("an evidence surface, not telemetry"). Every operator-visible latency is therefore bounded by the poll, and the REST cost grows with tabs × rooms, not with events.
2. The operator **Listen** button (LiveKit/Jitsi-style: hear a line the floor is not granting) was never built.

The wave-1 close (plan-7) left a usable asset for both: **the audit trail is already an event stream** (`audit.rs` `AuditEvent` → JSONL with `seq`, `dropped` counter). The push channel is a second subscriber to a stream that already exists — not a new state machine.

## 1. What (scope statement)

**In scope**

* **W1** — `/admin/ws`: one WS endpoint on the admin plane, authenticated by the **existing** `validate_admin` path, carrying a new `AdminEvent` tagged enum (snake_case, its own `v`), fed by a bounded fan-out from the audit emitter.
* **W2** — Console: Monitor becomes push-driven with a slow **reconcile poll** as the safety net (R2=A); Logs refetches on a debounced hint (R3=B); a stale-connection indicator (never silent staleness).
* **W3** — Comment/contract hygiene found on the way: the misleading SDK note about consumability (§5, last item).

**Out of scope** (stated so nobody discovers it mid-implementation)

* **Listen — deferred by R4=A**, with its four prerequisites listed so re-opening it is a checklist, not a re-investigation (§5).
* Any change to `SignalingMessage` / the user-side wire: the 27-variant contract and the R1 snapshot-resync model stay frozen (§2 explains why that is achievable).
* `muted` riding a floor message (plan-7 S7.5 chose the snapshot-push mechanism; re-opening it here breaks the "no new variants" ruling for no user gain).
* `audit.rs` **seq handling**: unchanged (R3=B removed the need — see §3.1). The file itself is not untouched: `AuditTx` gained an optional hub sink and `emit` publishes a hint after the ledger send (§3.3). Corrected after implementation, because "audit.rs is not touched" was only ever true of `seq`.
* GAP-3 secret rotation, GAP-4 `audit.jsonl` ops story, GAP-5 Alpha ops doc home, GAP-6 Alpha gate consolidation (D24 backlog — Alpha ops, not control plane).
* Drain / keys / limits UI, room tree, multi-line monitoring (still Alpha); Electron shell (D10) and SDK npm publish (D14): trigger-gated.

## 2. The wire question, answered before any code

The D23 annex assumed Listen and AdminEvent "require wire AdminListen/Role" additions to the **signaling** enum. Reading the auth code says otherwise, and the difference is the whole shape of this wave:

* `auth.rs:95-104` — `AdminClaims` **deliberately has no `room` field**, because `/ws` decodes `Claims` (room mandatory) in its two-pass flow: "an admin token structurally cannot pass — audience isolation survives even under the misconfiguration of both secrets being equal". Documented as a normative property in modules/02 §3 (written during the plan-7 §8 sync).
* Consequence: **AdminEvent cannot ride `/ws` without undoing that property.** And it does not need to: `/admin/ws` is the same out-of-band posture D18 chose for `/admin/api/*` ("zero additions to the signaling wire, zero media-plane change"), just with a socket instead of a request.
* So plan-7 §4's "NO new `SignalingMessage` variants" ruling **survives wave-2 unchanged** — no re-adjudication needed, contrary to what `.agents/memorys/status.md` Next Steps item 3 asserted until this plan corrected it.
* Compat note, measured rather than assumed: the browser SDK's reducer has a `default:` arm that returns the mirror unchanged (`packages/client/src/store.ts:119-124`), so an SPA predating a new `AdminEvent` variant ignores it. The strict case is a **typed Rust consumer** deserializing `SignalingMessage` — which is exactly why the admin enum is a separate type on a separate endpoint: adding a variant there can never break `/ws` clients.
* `api-interface-design`'s breaking-change ladder (version the endpoint `/ws/v2`, deprecation window) does **not** apply: nothing existing changes shape. `AdminEvent` ships with its own `v` so the admin plane can evolve on its own clock.

## 3. Contracts (the deliverable of this phase, not the code)

### 3.1 `AdminEvent` (new enum, `crates/talkservo-server/src/admin_events.rs`)

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdminEvent {
    /// v = admin-plane contract version (independent of SignalingMessage v:1)
    /// R3=B: the audit row itself is NOT pushed. "A new line exists" is enough —
    /// the console fetches /admin/api/audit?after=<its own max seq>, so the Logs
    /// table keeps exactly ONE source of content: no second row builder, no drift,
    /// and audit.rs keeps assigning seq inside the writer task (unchanged).
    AuditHint { v: u32 },
    /// R2=A: the live state IS pushed (Monitor would otherwise repaint nothing);
    /// the 30s reconcile bounds any loss, so a dropped event is a delay, not a lie.
    /// Payload source: the existing `rooms_live` serializer (`admin.rs:146-160`) already
    /// builds `Vec<serde_json::Value>` — the hub forwards that same body, so the push and
    /// the REST poll cannot disagree about shape.
    RoomsLive { v: u32, rooms: Vec<serde_json::Value> },
    /// on connect / after `subscribe`; also the health signal. `dropped` is the
    /// hub's own count of subscribers it had to evict (R5: bounded capacity).
    Hello { v: u32, dropped: u64 },
}
```

Client → server: `{"type":"subscribe"}` and `{"type":"ping"}` — no cursor travels upstream, because the client keeps its own max seq and asks REST for the delta. **State writes stay REST** (`/admin/api/*`), so the audit trail remains the single write path and no second mutation door appears.

### 3.2 Endpoint + auth

* `GET /admin/ws` (upgrade). Auth: the same `Bearer <admin jwt>` header as REST, validated by `validate_admin` with `TS_ADMIN_SECRET`; **no new claim field, no new role** — the tier model (S5) already lives in `AdminClaims.role`, and the stream is read-only so tiering does not gate it. 401 before upgrade ⇒ plain HTTP response (a browser `WebSocket` cannot read the body, so the console treats a failed upgrade as "fall back to reconcile + say why").
* Fail-closed posture unchanged: `TS_ADMIN_SECRET` unset ⇒ 503, same as the REST guard.
* Rate (R5=A): connects get their **own** window, a constant in code — not a config key (nobody has asked to tune one, and a key costs `config.rs` + modules/06 + facade + doctor sync). Sharing `admin_rate` with login/token issuance would let a post-restart reconnect storm from several tabs starve the very actions the console exists for, and the symptom (401 bounce) would point at the wrong cause. One socket per console tab is the accepted shape; cross-tab `BroadcastChannel` dedupe was rejected as a browser-side protocol that is barely testable, for a single-operator LAN.

### 3.3 Fan-out topology (the only real engineering risk)

```
mutation site ──► AuditTx.emit(ev)          (unchanged: one producer)
                        │
              mpsc ─────┴──────► JSONL writer (task, seq, rotation)   ← ledger, source of truth
                     └─────────► EventHub, bounded (tokio::sync::broadcast) ← /admin/ws subscribers
```

* `emit` stays non-blocking; the hub gets a `try_send`-style send and a lagging subscriber is **evicted with a counted drop**, never allowed to backpressure the ledger. The audit file must not lose a line because a browser was slow — that is the one property this slice must not trade away.
* The hub is a separate sink with its own dropped counter, surfaced in `Hello.dropped` and the Logs warning.
* No `seq` crosses the hub at all, so the two sinks cannot race on ordering and "seq gap" cannot be a false alarm (the R6 worry, designed away by R3=B).
* A burst of mutations must not become a burst of fetches: the console debounces `AuditHint` into one windowed refetch (~100 ms). A debounce is by definition a new silent surface, so §7 pins it with an explicit test.

## 4. Behavior contracts the console must keep

* **Push is a hint, the file is the ledger** (R3=B implemented literally): Logs never learns a row's content from the socket. A hint triggers a debounced incremental `?after=` fetch.
* That fetch path does **not** exist yet on the client side, and the plan does not pretend otherwise: `adminApi.audit()` (`web/src/lib/adminApi.ts:188`) takes only `event/room/actor/limit`, and the Logs tab full-refetches `limit: 100` (`web/src/pages/admin/logs.tsx:33`). The server side is already there (`AuditQuery.after` filtered at `audit.rs:240,268`, covered by a test at `audit.rs:355-361`). Adding the `after` param + a max-seq watermark is therefore **W2 scope, not a discovery to be made mid-implementation** (found by review, not by surprise).
* **Deltas for speed, snapshot for truth** (R2=A): Monitor applies `RoomsLive` immediately and keeps a 30 s reconcile. That poll is also today's *error* path (`setPollError` → Alert), so a quietly dead socket cannot make the UI stale without saying so.
* If the admin WS is down, Monitor shows the same Alert channel and the reconcile continues. "Green but not looking" is the failure mode this wave exists to remove; reproducing it in the new channel would be worse than the poll.
* The 401/503 → gate bounce (`admin.test.tsx`) carries over to the socket path: a bounced console must close the socket.
* Nothing in the UI may guess on-air state from the push; grants remain server-driven (modules/08 one-state law; S7.5's roster-vs-refusal wording stays exactly as adjudicated).

## 5. Listen — deferred by R4=A, with the real blockers named

D23 recorded Listen as "requires wire AdminListen/Role + SFU admin consume path". Both halves of that are wrong, and the truth is that **two existing gates** block it:

* **R-F14 consume gate** — `ws.rs:546-563` refuses any `Consume` whose producer's owner is not currently granted (`producer_owner_granted`, `room.rs:447-457`), answering `Error{code:"consume_denied"}` for floor-confidentiality reasons ("guessed ids get nothing"). There is no missing consume path; there is an explicit denial.
* **D13 uplink pause** — `apply_floor` pauses at **producer** level (`mediasoup_host.rs:358-375` diffs `state.grants()`). A paused producer carries no RTP, so even a permitted consumer of a non-granted line receives silence. Making a line audible to *only* the operator therefore also needs per-consumer pause control in the other direction — an axis the current diff does not have, and without it "monitor" means "everyone hears it".

Rejected alternatives, recorded so they are not re-derived:

* **L1 listen to granted lines only** — worthless: the dispatcher page already consumes granted speakers (`packages/client/src/media.ts` listen sweep), so the button would do nothing.
* **L2 shadow uplink / resume for monitoring** — requires formally weakening **acceptance #12** (`docs/architecture.md:77`: "non-holders produce nothing (server-paused) — verify zero idle uplink payload bytes per joined-silent peer except RTCP/consent"), plus the role, plus the reverse-pause axis. Four subsystems for one button, in a PoC with one operator.
* **L3 activity meter instead of audio** — a decoy: `AudioLevelObserver`/`last_activity` (`room.rs:47-49,473-490`) read zero for non-holders *by design*, which is exactly what #12 guarantees, so the meter answers "is that line silent?" with "yes, as intended" and slides into L2.

**To re-open Listen, these four are the price:** (1) an audited monitoring role/claim (a D16 surface), (2) a formal amendment to acceptance #12 including how it is measured, (3) per-consumer pause control in the SFU diff, (4) operator-visible, audited enter/exit for being overheard. The frozen dossier's own precedent points the other way anyway: LiveKit's hidden observer is a **read-only participant, not an admin bypass** (`console-feature-checklist-2026-10-19.md:139`).

Hygiene fix included in W3: `packages/client/src/media.ts:162-165` reads "LISTENING needs no grant — … Every announced producer is consumable", which invited precisely the wrong inference (mine, this session); the parenthetical about the server re-checking the gate stays, the headline gets qualified to name R-F14.

## 6. Layers / features affected (explicit, no "maybe")

| Layer | Path | Touched |
|---|---|---|
| Domain core | `crates/talkservo-core/` | **no** — `apply()` purity and the wire contract untouched (that is the point of §2) |
| SFU host | `crates/talkservo-sfu/` | **no** (Listen deferred ⇒ no consume/gating change) |
| Server | `crates/talkservo-server/` | yes — new `admin_events.rs` + bounded `EventHub`, `admin.rs` ws route + its own connect window, `ws.rs` App fields, `main.rs` + `test_bridge.rs` hub spawn and refresher. `audit.rs`: hub sink only (field, `spawn_for_with_hub`, one line in `emit`); **`seq` stays stamped in the writer task** (R3=B) |
| Web console | `web/src/pages/admin/`, `web/src/lib/adminApi.ts` | yes — event client with debounce + push-then-reconcile state + stale indicator |
| TS SDK | `packages/client/` | comment only (W3 hygiene); **no** admin code enters the SDK (D14 scope: signaling domain) |
| Cargo features | `stub-media` / `sfu-mediasoup` lanes | W1/W2 are **feature-independent** (no media plane); must build and pass under both, and under `embedded-web` |
| Config | `config.rs` / `server.yaml` / facade | **no new key** (R5=A: budget is a constant); none added unless an operator asks |

## 7. Tests (what "verified" means here)

1. **Rust unit**: `AdminEvent` serde shape (snake_case tag, `v` present, `AuditHint` carries nothing but `v`).
2. **Rust integration** (`tests/admin_events.rs`): subscribe → mutate (mute/kick) → receive `AuditHint` **and** the matching line in `audit.jsonl`; a never-reading subscriber is evicted at the hub bound without losing a file line (`dropped` counted, `Hello.dropped` reflects it); budget exhausted → upgrade refused, audit unaffected; unauthenticated / **user**-token upgrade ⇒ refused (extends the audience-isolation rows in both directions).
3. **vitest**: hint triggers exactly one debounced refetch for a burst (the debounce is asserted, not assumed — an untested timer is the new silent surface); push applies to Monitor state; socket down ⇒ reconcile continues and staleness is visible.
4. **e2e (Playwright, `TS_ADMIN=1`)**: two console tabs, action in one reflects in the other **without a poll cycle** (assert on the push; keep measuring the delta so the claim is numeric); a killed socket degrades visibly and recovers inside one reconcile window. **PIT-25** discipline: repeated presses wait out the request cooldown. **PIT-24**: no `getByRole(name)` in interaction paths. **PIT-26**: if a roster/push row flakes, capture WS frames + the `__tsSessions` mirror dump; no retry masking (`retries: 0` stands).
5. Lanes: `pixi run test` (ctest `-L "rust|web"`), `pixi run gate`, `pixi run e2e`.

## 8. Docs sync obligations (gate items, not optional)

* `docs/reference/server-api.md` — `/admin/ws` row (upgrade, auth, 401/503, budget) + the fan-out posture with its own dropped counter.
* **new** `docs/reference/admin-events.md` (living reference, C2): the `AdminEvent` contract table + version policy + the "hints never carry ledger rows" rule.
* `docs/modules/08-web-ui.md` — console surface: push + 30 s reconcile, stale indicator, debounced Logs; the Listen deferral with its four prerequisites (so the design doc stops implying it is one field away).
* `docs/architecture.md` §4 — wave-2 acceptance row (push latency measured; the "green but not looking" prohibition), filled in **only after** measurement.
* `packages/client/src/media.ts` — the consumability comment (§5, W3).
* `.agents/memorys/status.md` — the correction already applied this session stays; W4 records implementation state, and any new ledger entry updates its index rows in the same commit (C4 bites: it caught me mid-flight this session).

## 9. Adjudication record (user-adjudicated one by one, 2026-10-21)

| # | decision | outcome | key note |
|---|---|---|---|
| R1 | change name | *wave-2 realtime control plane* kept | rename is cheap now, invisible later |
| R2 | fate of the 3 s poll | **A — push + 30 s reconcile** | the poll is also today's only tested error path; deltas for speed, snapshot for truth |
| R3 | what Logs gets | **B — hint only, content always fetched** | one source of content ⇒ no drift; and it dissolved R6 |
| R4 | Listen | **A — deferred out of wave-2** | blocked by R-F14 + D13/#12, not by a missing wire field; four prerequisites recorded |
| R5 | sockets & budget | **A — one socket per tab, own connect budget as a constant** | sharing `admin_rate` lets a reconnect storm lock the operator out |
| R6 | seq before fan-out | **moot (dissolved by R3=B)** | `seq` untouched; the file still gained the hub sink |

**Ready for implementation on the user's "go".** W1+W2 (+W3 hygiene) is a complete shippable wave; its only hard part is sink isolation (§3.3). Keeping Listen out is what stops this wave from spending its risk budget on an accepted criterion (#12) instead.

## 10. R7 — found while landing W1 (needs a ruling before anyone "fixes" the test)

Landing the push plane put a probe into the cross-client kick row, and the probe answered the
week-old PIT-26 question: the witness page really does see `peer_joined(victim)` →
`peer_left(victim, kind:"removed")` → **`peer_joined(victim)` again** seconds later, with the
server log showing a real second `join`. `packages/client/src/signal.ts:168-172` reconnects on
any remote close, and the only terminal codes are `expired_token` / `invalid_token` (`:152-157`).
So **a kick cannot outlast the target's own valid token**, and the S7.4 e2e row fails
intermittently on it — a design gap, not a flaky test. Status: W1 is green on its own lanes;
`pixi run e2e` is intermittently red on this row until R7 is decided.

| | what it means | cost | blast radius |
|---|---|---|---|
| **R7-a** | accept it: kick = remove now, rejoin expected while the token lives; the console must show the return | no protocol change; the e2e row reworded to assert the return | operator-visible honesty; the action itself stays weak |
| **R7-b** | make the kick terminal for the client: a dedicated close code the SDK treats like `expired_token` | ~2 files, no new server state | stops only the AUTOMATIC return — a determined peer reloads with the same token |
| **R7-c** | refuse re-entry server-side: a (room, peer) removed-window gate on Join | real effect; new state + a TTL knob + its own audit event | a de-facto ban list the stateless-JWT design (D22) has been avoiding |

**RULING (2026-10-21): R7 = B, implemented.** The middle sentence was:

> b is the
smallest change that stops the operator's action from lying to itself, and c introduces
membership state this project has deliberately not carried.

Landed as: `RoomCommand::Remove{peer, reason}` → close `4410` (terminal in `signal.ts`, with a
`removed` event carrying the operator's words) → field/participant outrank offline/mute/refusal
wording with it; `Left` keeps `1000` so R1 retry still covers real drops. Pinned at three layers
(SDK contrast tests, web copy test, the e2e row that started this). R7-c remains available if the
beachhead later needs "stay out" — it would need its own audit event and a D22 revision.

