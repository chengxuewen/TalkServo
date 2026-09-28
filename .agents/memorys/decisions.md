# TalkServo Architecture Decisions

> Format: `## D{N}: Title` — decision + date + rationale + impact (+ Supersedes/Limits if any).
> Numbering starts at D1 and increases consecutively.
>
> Note: the documentation system was ported from MediaServo on 2026-09-28; its D-series history is not migrated.
> When MediaServo's existing decisions are relevant, read its repository's `.agents/memorys/decisions.md`;
> if this project adopts a similar approach, file a new D-entry here citing the source.

<!-- First decision starts here -->

## D1: Floor abstraction unifies PTT / full-duplex / hybrid modes (2026-09-28)

**Decision**: One primitive — the Floor — with modes `Exclusive` (PTT), `Open` (conference), `Hybrid` (dynamic switching). All modes are expressed as floor state transitions plus grants; no parallel subsystems.
**Date**: 2026-09-28
**Rationale**:
- Whitepaper thesis: existing OSS covers one mode each; a unified model avoids a split brain between "intercom" and "conference" code paths
- State machine + priority/pre-emption semantics live in one place (`talkservo-core`), testable without I/O
**Impact**: Server, SDK, and client share one protocol vocabulary; new modes are `FloorMode` variants, not new services.
**Source**: whitepaper.md §5, §12 (user-provided planning draft)

## D2: Centralized signaling + SFU media, separate processes, signaling never mixes audio (2026-09-28)

**Decision**: WebSocket signaling server arbitrates floor; a separate SFU process forwards RTP only. Floor transitions are pushed to the SFU as grant/revoke control data; the SFU holds no arbitration logic.
**Date**: 2026-09-28
**Rationale**:
- Failure isolation and independent scaling by process boundary
- PTT-to-audible latency = one signaling RTT + SFU switch, not a session renegotiation (audio track stays published; muting is relay-side)
- MediaServo lesson: ICE must be explicit even on loopback; STUN/TURN (coturn) is mandatory from day 1
**Impact**: two-process PoC topology; all-in-one binary allowed for dev convenience only.
**Source**: whitepaper.md §6, architecture.md §3.2

## D3: webrtc-rs as media engine; Google libwebrtc not adopted (2026-09-28)

**Decision**: Pure-Rust WebRTC stack (webrtc-rs) for the SFU and native clients. The relay is composed from webrtc-rs `broadcast`/`rtp-forwarder` primitives — verification 2026-09-28: the webrtc-rs org ships a sans-IO `sfu` building-block crate but **no complete SFU example** (correcting the planning-discussion premise of a reusable official SFU crate); `pion/ion-sfu` (moved to ionorg, inactive since 2023-07-21, not GitHub-archived) serves as architecture reference. libdatachannel noted as the C++-ecosystem alternative; libwebrtc rejected for weight.
**Date**: 2026-09-28
**Rationale**:
- Language alignment with the Rust core; no FFI boundary in the media path itself
- Known gap: webrtc-rs lacks NetEQ/full 3A — accepted, compensated via D4 and receiver-side FEC/PLC
- Verified 2026-09-28: active maintenance (5.1k stars, pushed the day before snapshot, 5 open issues); no complete SFU example (org's sans-IO `sfu` crate is a building block) — relay is ours to compose
**Impact**: FEC recovery and jitter handling are our code (manual Opus `fec=true` second decode); productization may re-evaluate pure-Rust 3A (aec3-rs/sonora).
**Source**: whitepaper.md §7.1-7.2

## D4: 3A via webrtc-audio-processing FFI in PoC (2026-09-28)

**Decision**: The single C++ dependency allowed in the tree during PoC is `webrtc-audio-processing` (Google AEC/AGC/ANS via FFI). Pure-Rust or composed-crate alternatives deferred to productization evaluation.
**Date**: 2026-09-28
**Rationale**:
- Production-proven audio quality is the fastest path to a usable voice demo
- One controlled FFI seam; isolating it keeps the rest of the tree pure Rust
**Impact**: Linux-first CI for the FFI path (mirrors MediaServo's Docker/macos split experience); feature-gate so platforms without C++ toolchains get a passthrough stub.
**Source**: whitepaper.md §7.3

## D5: Crate split talkservo-core / -server / -sfu / -client; default features must build everything (2026-09-28)

**Decision**: Four crates as in architecture.md §3.1. `talkservo-core` has no async/I/O. All PoC-required functionality is in **default features**; backends (`sfu-webrtc` / `stub-media`) are compile-time feature gates, never runtime `--features` flags for core builds.
**Date**: 2026-09-28
**Rationale**:
- Pure functional core = exhaustive unit tests of arbitration without tokio
- MediaServo inherited lesson (feature-flag discipline): docs/CI must not carry `--features` for the default build path
**Impact**: CI matrix linux (full) / macos (no-sfu); binding crates (UniFFI/wasm) attach to `talkservo-client` later.
**Source**: architecture.md §3.1, repo rules (Feature Flag Discipline)
