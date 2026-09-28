# Error Model
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Failure matrix

Principles: media failure never corrupts arbitration state; clients never self-heal by guessing (server broadcasts recovery); every error log carries `room/peer/generation`.

| # | Failure | Detection | Floor impact | Recovery | UX |
|---|---------|-----------|--------------|----------|----|
| E1 | holder WS drop | heartbeat 30 s / TCP half-open | release + auto-promote | R-sequence on reconnect | muted notice → restored |
| **E11** | holder **media dies, WS alive** (mic revoked, 4G handover, silent uplink) | `iceConsentTimeout` 30 s auto-close; `icestatechange('disconnected')`→close; PoC adds no-RTP watchdog (2 s) | `MediaDown` after `floor_media_grace_ms` → release+promote (D12) | F-sequence | all see `MediaFailed{holder}` + new grant |
| E2 | reconnect race | stale `gen` discard | none | — | seamless |
| E3 | mic denied/busy | getUserMedia reject | none | client shows banner; no Request sent | explicit |
| E4 | transport timeout (mediasoup 10 s) | await err | none | retry 1 → kick media half only, `MediaFailed{peer}` | "peer audio unreachable" |
| E5 | ICE all-fail | dtls failed | none | as E4; ops checks TURN/TCP | as E4 |
| E6 | **worker crash** | `worker.exited` | **none** | W-sequence rebuild (~2-4 s est., measure) | brief global silence, auto-restore |
| E7 | server process death | supervisor | lost (PoC: no persistence — accepted) | full reconnect + empty-room rebuild | brief total drop |
| E8 | forged/invalid msg | apply validation → Denied{NotMember} | rejected | warn-log (security surface #1) | sender sees Denied |
| E9 | JWT expired | Join reject | — | close 4401; client refresh+rejoin | re-auth |
| E10 | port pool exhausted | transport create err | no new media | `MediaCapacity`; existing sessions untouched; PoC cap ~50 transports | "room full" |

On startup `generation` begins at a random high offset (E7 stale-socket guard, review F-12). Explicitly NOT handled in PoC (YAGNI, revisit at Alpha): FloorState persistence, cross-worker migration, per-message ACK, clock sync (generation replaces wall-clock ordering), rolling-upgrade/version-rollout policy (lands with Alpha deployment.md).

## Observability (PoC: structured logs only)

No metrics server, no tracing stack in PoC (ponytail: logs suffice for acceptance measurement; Prometheus/OTel deferred to Alpha ops scope). Requirements:

- transport: `tracing` with JSON formatter to stdout; level policy: info=protocol events, warn=denials/E-class, error=E6/E10
- mandatory fields on every line: `room`, `peer`, `gen` (the §1 principle), plus `event`
- event vocabulary (subset closed for parsing): `join leave floor_request grant deny taken release idle_timeout mode_change transport_create transport_fail produce_ok consume_ok worker_exited media_restart_done rate_limited`
- acceptance measurements read from logs: `grant_latency_ms` = t(grant) − t(floor_request) same room/peer; `media_recovery_ms` = t(media_restart_done) − t(worker_exited) (E6 #7); P-latency uses client-side marks (§ modules/04 harness), logs cross-check ordering via `gen`
