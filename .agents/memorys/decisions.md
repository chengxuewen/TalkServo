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
**Impact**: two-process topology as steady state; PoC form re-scoped by D7 to one supervised binary (see D7 Limits).
**Source**: whitepaper.md §6, architecture.md §3.2

## D3: webrtc-rs as media engine; Google libwebrtc not adopted (2026-09-28)

**Decision**: Pure-Rust WebRTC stack (webrtc-rs) for the SFU and native clients. The relay is composed from webrtc-rs `broadcast`/`rtp-forwarder` primitives — verification 2026-09-28: the webrtc-rs org ships a sans-IO `sfu` building-block crate but **no complete SFU example** (correcting the planning-discussion premise of a reusable official SFU crate); `pion/ion-sfu` (moved to ionorg, inactive since 2023-07-21, not GitHub-archived) serves as architecture reference. libdatachannel noted as the C++-ecosystem alternative; libwebrtc rejected for weight.
**Note (2026-09-28, audit)**: media-engine scope SUPERSEDED by D6 (mediasoup); live scope = no-libwebrtc rationale + candidate engine note for future native clients. The self-composed-relay FEC impact no longer applies to the SFU (server-side FEC is relay-passthrough, docs/modules/04).
**Date**: 2026-09-28
**Rationale**:
- Language alignment with the Rust core; no FFI boundary in the media path itself
- Known gap: webrtc-rs lacks NetEQ/full 3A — accepted, compensated via D4 and receiver-side FEC/PLC
- Verified 2026-09-28: active maintenance (5.1k stars, pushed the day before snapshot, 5 open issues); no complete SFU example (org's sans-IO `sfu` crate is a building block) — relay is ours to compose
**Impact**: FEC recovery and jitter handling are our code (manual Opus `fec=true` second decode); productization may re-evaluate pure-Rust 3A (aec3-rs/sonora).
**Source**: whitepaper.md §7.1-7.2

## D4: 3A via webrtc-audio-processing FFI in PoC (2026-09-28)

**Decision**: The single C++ dependency allowed in the tree during PoC is `webrtc-audio-processing` (Google AEC/AGC/ANS via FFI). Pure-Rust or composed-crate alternatives deferred to productization evaluation.
**Note (2026-09-28, audit)**: approved PoC form (D7/D9) is web-only — D4 activates at first native capture (Beta), not literally during PoC; gate surface lands with `talkservo-client`.
**Date**: 2026-09-28
**Rationale**:
- Production-proven audio quality is the fastest path to a usable voice demo
- One controlled FFI seam; isolating it keeps the rest of the tree pure Rust
**Impact**: Linux-first CI for the FFI path (mirrors MediaServo's Docker/macos split experience); feature-gate so platforms without C++ toolchains get a passthrough stub.
**Source**: whitepaper.md §7.3

## D5: Crate split talkservo-core / -server / -sfu / -client; default features must build everything (2026-09-28)

**Decision**: Four crates as steady-state target (docs/modules/03-components.md; PoC scope = 3 crates per D7). `talkservo-core` has no async/I/O. All PoC-required functionality is in **default features**; backends (`sfu-mediasoup` / `stub-media`; literal corrected post-D6 — was `sfu-webrtc` at adoption) are compile-time feature gates, never runtime `--features` flags for core builds.
**Date**: 2026-09-28
**Rationale**:
- Pure functional core = exhaustive unit tests of arbitration without tokio
- MediaServo inherited lesson (feature-flag discipline): docs/CI must not carry `--features` for the default build path
**Impact**: CI matrix linux (full) / macos (no-sfu); binding crates (UniFFI/wasm) attach to `talkservo-client` later.
**Source**: docs/modules/03-components.md, repo rules (Feature Flag Discipline)

## D6: SFU media engine = mediasoup (Rust crate 0.24), superseding D3's engine choice (2026-09-28)

**Decision**: User ruling during PoC design brainstorm (2026-09-28): the server media layer uses the `mediasoup` Rust crate v0.24 (spawns the mediasoup-sys C++ worker subprocess; Router/WebRtcTransport/Producer/Consumer model; ICE-Lite server side) — same binding and version lineage as sister project MediaServo (D138, feature `sfu-mediasoup`). Replaces D3's webrtc-rs self-composed relay for the SFU; the "no embedded libwebrtc" rationale of D3 stands (mediasoup worker is a separate process, not linked libwebrtc). D1/D2/D4/D5 unaffected; D3's native-client webrtc-rs scope deferred (no native client in PoC).
**Date**: 2026-09-28
**Rationale**:
- Floor grant/revoke maps 1:1 to Producer/Consumer pause/resume/creation — audibility is router state, not client mute
- Production-proven router isolation, consumer fan-out, ICE-Lite; multi-SFU (OQ-4) has a documented path
- MediaServo expertise, CI layout (test-mediasoup ubuntu-only), Docker workflow and the ported platform.md/docker.md rules become live assets
**Costs accepted**: Linux x86_64 build constraint for the SFU feature (macOS = check-only); worker subprocess lifecycle + crash recovery on us; first Docker build 15-30 min.
**Supersedes**: D3 (media-engine portion only; D3 kept for its no-libwebrtc rationale + future native-client note).
**Source**: user instruction 2026-09-28; MediaServo docs/modules/sfu-mediasoup-integration.md + Cargo.toml (`mediasoup = 0.24`) verified this session.

## D7: PoC crate scope = 3 crates (core/sfu/server); talkservo-client deferred to first native consumer (2026-09-28)

**Decision**: PoC builds `talkservo-core`, `talkservo-sfu` (mediasoup host layer, elevated from server submodule because worker lifecycle is a real concern), `talkservo-server` (binary). `talkservo-client` is NOT created until the first native/SDK consumer appears (Beta trigger); the web client uses the official mediasoup-client npm + our WS wire contract directly.
**Date**: 2026-09-28
**Rationale**: crate boundaries should be earned by real consumers (D5 discipline extension); mediasoup-client reimplementation in Rust is unnecessary in PoC and its protocol surface is owned upstream.
**Limits**: D5's 4-crate target remains the steady-state topology; D7 is a staged scope, not a reversal. D7 also re-scopes D2's Impact clause: PoC topology = one supervised binary; the split becomes physical at Alpha.
**Source**: PoC design brainstorm §1-§6, user approvals 2026-09-28.

## D8: SDK strategy — single session facade, C-ABI base, trigger-grown bindings, nothing in PoC (2026-09-28)

**Decision**: TalkServo SDK = one Rust `talkservo-client` session facade → C ABI base (`talkservo_` prefix, soname/ABI discipline) → thin C++ RAII / Python ctypes wrappers; mobile via UniFFI (Kotlin/Swift). Node.js bindings NOT planned absent a named consumer. Binding CI jobs capped at 3. PoC writes no SDK code (wire contract is browser-complete). Native media engine phased c→b per OQ-9.
**Date**: 2026-09-28
**Rationale**: MediaServo's four-SDK/wide-matrix precedent proves the cost (D65-D69→D222-D234 full API rewrite, 4 permanent binding CI jobs); floor/arbitration being server-side makes the client facade thin and language-portable; C-ABI-as-contract is the sister project's verified layering (D227 family).
**Limits**: does not supersede D7 (PoC crate deferral) — it explains what the deferred crate will be when first consumer arrives (Beta).
**Source**: user ruling 2026-09-28 (brainstorm SDK analysis); MediaServo docs/modules/04-sdk-layers.md + 23-binding-guide.md verified this session.

## D9: Web UI PoC scope — two role screens, single room, admin deferred, gap ledger accepted (2026-09-28)

**Decision**: PoC web = SPA with `/d/:room` dispatcher (target grid + tally rings + event log + per-line mute + grant-queue strip) and `/f/:room` field (roster + oversized hold-to-talk + keybind/release-delay drawer). Stack: Vite+React+TS+AntD5+zustand, rust-embed delivery, mediasoup-client npm, no zod/charts. Admin surfaces (keys/settings/logs/live monitor UI) = Alpha backlog with the object-spine from the admin dossier; on-air state is server-driven only (client never guesses floor). Hard browser gaps (OS-global hotkeys, background receive) accepted and assigned to D8 native shell; single-room deferral of multi-line monitoring accepted as the largest conscious gap.
**Date**: 2026-09-28
**Rationale**: 20-line gap ledger vs mainstream (8 parity, 1 differentiator, 2 hard gaps, rest deferred with owners); vertical-slice discipline keeps admin out of PoC since dispatcher grid already exposes floor truth.
**Source**: brainstorm §12 approval + ui research dossiers (ptt-ui-patterns, admin-ui-patterns).

## D10: First-party desktop = Electron shell over the web SPA (2026-09-28)

**Decision**: User ruling: desktop dispatch console ships as Electron wrapping `web/` unchanged — renderer keeps Chromium WebRTC + mediasoup-client (behavior parity with browser acceptance; Discord-desktop precedent); main process adds OS integration only (globalShortcut closing hard gap #5 on desktop, tray, autostart, backgroundThrottling:false). No Rust client in the desktop path; Tauri rejected on WebView media fragmentation; costs (~90MB, CVE following, updater) accepted. Target Alpha; field mobile stays D8 native track. Node bindings: Electron names a *potential* future consumer (main-process needs), still trigger-gated, no pre-built bindings/node.
**Date**: 2026-09-28
**Source**: user preference + comparison analysis this session (Tauri's Rust-reuse advantage voided by server-side arbitration architecture, D1/D2).

## D11: Dev toolchain = pixi + two-stage bootstrap, Linux-native-first (Docker demoted to CI parity) (2026-09-28)

**Decision**: Adopt MediaServo's pixi workspace pattern (single lock for rust/python/toolchain/nodejs22, activation env injection incl. the MESON_ARGS-unset precedent, feature.dev/ci envs) and the bootstrap.sh → pixi.sh two-stage entry. Because the dev machine is Linux x86_64 — the mediasoup worker's native platform — day-to-day builds/tests of talkservo-server + sfu-mediasoup run NATIVELY; docker-cargo.sh exists only for CI-parity runs and deploy images. Rejected for now: GStreamer/flatbuffers deps, Jetson aarch64 activation, Windows .bat script layer, bindings task matrix (Beta trigger). scripts/ ships with workspace P0-1 (same vertical slice as Cargo.toml): _common.sh, bootstrap.sh, pixi.sh, docker-cargo.sh (backup), scan-hardcode.sh (fills the security skill's dangling check-command reference).
**Date**: 2026-09-28
**Rationale**: platform.md/docker.md's 'macOS dev + Docker server' shape was MediaServo's macOS-forced form, not a law; borrowing mechanisms while correcting premises avoids inherited complexity. nodejs in pixi (MediaServo missed it) makes web builds reproducible.
**Source**: analysis session 2026-09-28; details docs/modules/09-dev-toolchain.md.

## D12: FloorState v2 — grant-set model, MediaDown release, queue events, closed DenyReason (2026-09-28)

**Decision**: Post-review (architecture-review-consolidated #1·3·4·5·12·13): replace single-holder slot with `grants: Vec<PeerId>` (cap 1/2/N by mode incl. MCPTT dual-floor), explicit `muted` set, `FloorQueued{pos,gen}` + ModeChange/MuteSet wire messages, `MediaDown` event + `floor_media_grace_ms` release path (no zombie holders), closed `DenyReason` enum incl. RateLimited/PreemptPriority, generation boot offset, role-scoped `ServerSnapshot` (queue = dispatcher-only), `TokenRefresh` push (LiveKit pattern), identity-collision=reject.
**Date**: 2026-09-28 | **Amends**: D1 (presentation only — the abstraction holds), D2 impact wording via D13.
**Source**: arch-review-team 2026-09-28 (floor F-01..08, media M-3, client C-6/8/9); all fixes ratified by user approval 2026-09-28.

## D13: Transmission gating — server pauses non-granted producers; mono Opus; consumers via pause/resume diff (2026-09-28)

**Decision**: Refines M1: producers registered once but **paused unless granted** (server-side `Producer::pause`), consumers created-once-then-pause/resume-diff (unifies the three-way doc contradiction, review M-4). Codec pinned mono 48k fullband (M-10). Consequences: idle peers emit no audio uplink (privacy + mobile data + battery), P-budget rides sub-ms pause/resume, DTX question moot.
**Date**: 2026-09-28 | **Limits**: keep-alive RTCP/consent traffic continues while paused — zombie detection still owns watchdog duty (E11).
**Source**: media-reviewer M-1/M-2/M-4/M-10 + crate API verification (versatica/mediasoup rust tree, 2026-09-28).
