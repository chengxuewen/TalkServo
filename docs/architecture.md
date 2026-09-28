# TalkServo Architecture

> Phase 0-1 — architecture definition | v0.2 | 2026-09-28 | Source: [whitepaper.md](whitepaper.md) | Decisions: ledger D1-D11+ ([.agents/memorys/decisions.md](../.agents/memorys/decisions.md)) — cite by id, no range in this line (drift guard)
>
> **Revision log**
> - 2026-09-28 (D6): SFU engine = **mediasoup** (Rust crate 0.24, C++ worker subprocess), superseding D3's webrtc-rs self-composed relay. Frozen research snapshots keep their original recommendations as records.
> - 2026-09-28 (design session, this version): PoC scope agreed via brainstorm — beachhead **dispatch**, form **vertical slice**, reliability **R1 snapshot-resync**, network scope **incl. public/TURN**, FEC **in acceptance**. Sections 3-7 absorbed the approved design; D7 fixes PoC crate count at 3.
>
> Status: pre-implementation. Design detail lives in `docs/modules/` (living, updated in place); `docs/reference/research/` holds frozen evidence dossiers (C2).

## 1. Overview

TalkServo is a real-time voice **floor-control platform**: PTT (exclusive), conference (open), and hybrid modes are one abstraction — the Floor.

| Mode | FloorMode | Speakers | Use case |
|------|-----------|----------|----------|
| PTT intercom | `Exclusive` | exactly 1 (the holder) | **dispatch (PoC beachhead)**, emergency |
| Full-duplex conference | `Open` | all participants | team voice channel |
| Hybrid | `Hybrid` | dynamic, by role/priority | commander barge-in into channel |

Design principles:

| Principle | Meaning |
|-----------|---------|
| **Floor is the single primitive** | every mode = floor state + grants; no separate intercom/conference subsystem (D1) |
| **Pure-Rust core, I/O at the edge** | `talkservo-core` is a deterministic state machine, exhaustively unit-testable |
| **Centralized arbitration** | one authority per room; clients never self-grant (D2) |
| **SFU forwards, never decides** | audibility = mediasoup Router state driven by floor events |
| **Protocol-first** | the WS wire contract is a versioned artifact; SDKs wrap it |

## 2. Module Index (design docs, `docs/modules/`)

| Module | Covers | Source sections (v0.1 lineage) |
|--------|--------|-------------------------------|
| [01-floor-model.md](modules/01-floor-model.md) | state machine, FloorState/apply(), rules 1-8, priority | §2.1 |
| [02-signaling-protocol.md](modules/02-signaling-protocol.md) | WS enum wire contract, sequences J/P/X/R/W, R1 rationale | §2.2-2.3 |
| [03-components.md](modules/03-components.md) | crates, topology, Sfu trait, feature gates | §3 |
| [04-media-pipeline.md](modules/04-media-pipeline.md) | mediasoup config, codec policy, coturn, netem | §4 |
| [05-error-model.md](modules/05-error-model.md) | E1-E10 matrix, three principles, explicit non-goals | §5 |
| [06-deployment-security.md](modules/06-deployment-security.md) | deployment ladder, security posture | §6-7 |
| [07-sdk-strategy.md](modules/07-sdk-strategy.md) | facade+C-ABI binding strategy (D8) | §10 |
| [08-web-ui.md](modules/08-web-ui.md) | PoC screens, floor-viz law, stack, gap ledger (D9) | §11 |
| [09-dev-toolchain.md](modules/09-dev-toolchain.md) | pixi/bootstrap toolchain, Linux-native-first posture (D11) | new |

> Split 2026-09-28 per user ruling: design docs live in `modules/` (living, updated in place as implementation lands); `reference/` holds research dossiers & living API/config handbooks only. `docs/research/` no longer exists (moved under reference/ earlier).

## 3. Open questions

| # | Question | Stage | Notes |
|---|----------|-------|-------|
| ~~OQ-1~~ | closed by D6 (mediasoup) | — | frozen sweeps retained |
| OQ-2 | MQTT secondary transport | no | WS-first |
| OQ-3 | holder recording boundary | post-PoC | server-side Consumer→recorder tap is mediasoup-native |
| OQ-4 | multi-SFU routing | Alpha | mediasoup multi-worker/pipe paths documented; MediaServo experience applies |
| OQ-5 | in-room text chat | no | YAGNI until asked |
| OQ-6 | **closed (R1)**: snapshot-resync over TCP signaling; MCPTT re-send timers are for unreliable signaling paths — revisit only if UDP signaling ever appears | — | recorded in modules/02 §2 |
| OQ-7 | IPv6 / v4-v6 dual-stack candidates | Alpha | PoC IPv4-only |
| OQ-8 | TCP transport throughput profile (4G relay over TCP) | acceptance data | enabled in PoC, measure don't assume |
| OQ-9 | **native SDK media engine**: (c) platform-native WebRTC in mobile shells + Rust owns signaling/floor only → (b) `webrtc-sys` prebuilt libwebrtc FFI (MediaServo-proven) if (c) quality floor unmet; (a) pure webrtc-rs+D4 stays last-resort — mobile audio I/O/NetEQ unproven. Would supersede D3 if (b) chosen | Beta (first native consumer) | phased posture c→b is D8; reopening requires new D entry |
| OQ-10 | E2EE/SFrame posture before any external hosting (D6 worker holds plaintext — modules/06 trust statement) | Beta gate | review M-1 |
| OQ-11 | multi-identity/displace policy (today: reject second Join same identity) | Alpha | review C-9 |

## 4. Acceptance (normative)

Functional (§9 of v0.1 retained): A holds → B/C denied; A audible to B/C; release → Idle; strictly-higher preempt works; holder drop → auto-release.

Additional (from design session):

6. R1: kill field peer mid-hold → reconnect → snapshot restores correct Idle/granted state, no double-grant.
7. E6: kill worker → W-sequence restores audio plane without touching floor state (measure real recovery time).
11. Zombie-holder (E11): kill holder uplink (mic revoked / no-RTP 2 s) with WS alive → `MediaDown` within grace → floor frees, queue promotes — floor NEVER locks on media death.
12. Transmission gating (D13): non-holders produce nothing (server-paused) — verify zero idle uplink bytes per joined-silent peer except RTCP/consent.
8. FEC A/B under netem: concealment-share drops ≥30% (FEC on), quantitative via getStats.
9. ICE classes: host / srflx / relay each complete one full P-sequence on public deployment.
10. P-latency: keydown→audible ≤ budget lines (300/600 ms) — measured, then this line revised with data.

Test layers: unit (core, any platform) → integration (stub-media) → e2e automated (Linux, real worker + Playwright fake-device 3-instance scenario) → manual public network. CI: fmt/clippy/test dual-OS + test-mediasoup ubuntu + nightly e2e (non-blocking) — reusing the ported ci-cd-automation skeleton.

## 5. Relationship to MediaServo

| Aspect | MediaServo | TalkServo |
|--------|-----------|-----------|
| Domain | multimedia infra (video/teleop/streaming) | voice floor control (dispatch first) |
| SFU engine | mediasoup-sys (same crate lineage) | **same engine (D6)** — ported platform/docker rules now live assets |
| Arbitration | none | the core primitive (D1) |
| SDP flow | Server-Offer (D198) | standard client-offer mediasoup-client npm in web PoC; revisit for native SDK |
| SDK shape | four facades (link/field/deck/client) × 4 binding langs, wide matrix | single client facade, ≤3 binding jobs, trigger-growth (modules/07) |

 > - 2026-09-28 (review remediation, D12/D13): arch-review team 21 findings — grant-set FloorState, MediaDown/E11 release, ModeChange/MuteSet/FloorQueued/TokenRefresh wire, DTX moot + mono, server-side producer gating, wss/TLS + trust statement, snapshot role-scoping, /healthz, config defaults table, pixi.lock rule; consolidated: reference/research/internal/architecture-review-consolidated.md
> - 2026-09-28 (structure): design sections carved into `docs/modules/` per user ruling — modules/ = design, reference/ = external knowledge. Master retains overview/index/OQ/acceptance/relationship.
