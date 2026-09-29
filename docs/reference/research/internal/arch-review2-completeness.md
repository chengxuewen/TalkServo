# Arch-Review Round 2 — Completeness Sweep (Internal)

> **FROZEN** — one-time self-review dossier (C2: research archives are never back-written; new findings get new dated files).
> Generated: 2026-09-28 · Scope: data/state lifecycles, contract versioning, capacity limits, ops leftovers, product-domain notes.
> Method: full read of docs/architecture.md + modules/01-09 + README indexes; each finding classified **GAP** (missing everywhere) vs **LINK** (defined in one place only, not carried through). No overlap with round-1 dossiers (`architecture-review-*.md`).

| ID | Severity | Class | Title |
|----|----------|-------|-------|
| CM-1 | HIGH | GAP | No room lifecycle: GC/TTL after last peer leaves is undefined |
| CM-2 | HIGH | GAP | Disconnected queued peer is never removed from the queue (Rule 8 makes it a no-op) |
| CM-3 | MEDIUM | LINK | PeerList emission semantics undefined (join broadcast, leave notice, delta vs full) |
| CM-4 | MEDIUM | LINK | SFU orphan cleanup on peer leave: `peer_left` seam named, behavior undefined |
| CM-5 | MEDIUM | LINK | Wire version `v:1` exists only in prose — no version field, no negotiation/reject |
| CM-6 | MEDIUM | GAP | No capacity limits: peers per room, queue depth, WS message size |
| CM-7 | LOW | GAP | Ops leftovers: no log rotation/retention/timestamp-format note |
| CM-8 | **PRODUCT-DOMAIN** | GAP | China dispatch market: PIPL / recording consent absent everywhere |

## Findings

**CM-1 | HIGH | room lifecycle.** Evidence: the only TTLs in the repo are JWT/TURN (docs/modules/06 §2, 3600 s). No doc defines room teardown after the last peer leaves; W-seq rebuilds "one Router per active room" (docs/modules/02 §2) and modules/03 registers a Router per room — nothing ever closes it (the round-1 phrase "state GC after TTL" no longer exists in modules/03). A long-running PoC server accumulates Routers/transports/memory per room forever. Recommendation: one sentence in modules/03 — room created on first Join, reaped at last leave + heartbeat grace, `router.close()`; add `ROOM_IDLE_TTL_S` to the 06 config-key list.

**CM-2 | HIGH | queue vs disconnect.** Evidence: `queue: Vec<Pending>` (docs/modules/01 §1.1) is pruned only by grant/promote/ModeChange; `Leave{peer}` with a non-granted peer is an "idempotent no-op" (Rule 8) — as written, a disconnected QUEUED peer stays in the queue and the next promote can grant the floor to a ghost. E1 covers holder WS-drop only; F-sequence covers holder MediaDown only. Recommendation: rewrite Rule 8 as "Leave removes the peer from grants **and** queue; absent peer = no-op", and prune the queue from the same WS-drop event that fires E1. Unit test: queue [A,B], B disconnects → next grant is A, never B.

**CM-3 | MEDIUM | PeerList semantics.** Evidence: `PeerList{peers}` sits in the server→client table (docs/modules/02 §1) but no sequence emits it: J-sequence delivers Welcome+RouterCaps only; client leave has no wire message at all ("leave = WS close" is nowhere stated); full-replace vs delta and frequency are undefined; ServerSnapshot carries `peers` only at (re)connect. Recommendation: define in modules/02 §2 — broadcast full-replace PeerList to all room members on join and on leave detection (PoC room sizes make deltas pointless), and state the leave-detection rule explicitly.

**CM-4 | MEDIUM | SFU orphan cleanup.** Evidence: `Sfu::peer_left(room, peer)` exists in the trait (docs/modules/03) but its contract appears nowhere: no doc says WS drop closes the peer's WebRtcTransport (which cascades to owned producers/consumers in mediasoup), nor who keeps consuming a departed peer's producers. Recommendation: one line in modules/03 — `peer_left` closes the peer's transports (mediasoup auto-closes owned producers/consumers); `apply_floor` diffs away the resulting consumers; CM-1's reap catches stragglers.

**CM-5 | MEDIUM | wire versioning.** Evidence: "v:1" appears only in the modules/02 §1 heading; no message carries a version field; `Join{jwt}` declares no client version; server behavior on an unknown `type` (serde tagged-enum deserialize failure) is unspecified — "unknown fields ignored in release" covers fields, not types. Recommendation: PoC-minimum: any undeserializable message → `Error{BadVersion}` + close, so old/mismatched clients are never silently hung; optional one-liner: server declares `v` inside `Welcome`, making the "versioned artifact" claim (docs/architecture.md §2 Protocol-first) true on the wire.

**CM-6 | MEDIUM | capacity limits.** Evidence: the 06 §2 config-key list has no peer cap, no queue bound (`Vec<Pending>` unbounded), no WS frame-size cap; E10's ~50 transports is the only number in the repo. An unbounded queue sits right next to E8 being named "security surface #1" (docs/modules/05 §1). Recommendation: three config keys at P0-1 — `MAX_PEERS_PER_ROOM`, `MAX_QUEUE_DEPTH`, WS max frame size — with sane defaults (e.g. 50 / 32 / 1 MiB); overflow → `Denied{Busy}` (the closed DenyReason enum already fits, no wire change).

**CM-7 | LOW | ops leftovers.** Evidence: 05 §Observability fixes JSON-to-stdout only; rotation/retention is unaddressed (docker/ lands at P0-1; the default json-file driver is unbounded on disk); timestamp format (ISO8601/UTC) is unspecified although acceptance metrics (`grant_latency_ms`) are read from log timestamps. Recommendation: one line in modules/05 — compose sets json-file `max-size`/`max-file`; tracing emits RFC3339 UTC. Zero code.

**CM-8 | **PRODUCT-DOMAIN** | compliance, China dispatch market.** Evidence: no PIPL / data-residency / recording-consent text anywhere in docs/ (the whitepaper touches only China *trademark* registration); research precedent PushComm ships "recording+CDR" as a standard dispatch feature, so the question will come. **PRODUCT-DOMAIN NOTE ONLY — intentionally not designed here**: PIPL (cross-border transfer, consent basis) and recording consent become gating questions before any China-market dispatch deployment; belongs to the product owner, not PoC docs.
