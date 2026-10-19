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

## D14: TS client SDK as first-class package — `packages/client`, PoC unreleased, npm publish at Beta (2026-09-28)

**Decision**: The client-side glue (WS lifecycle, floor-state mirror, mediasoup-client orchestration, R1 recovery, TokenRefresh, role-scoped dispatch) lives in a dedicated workspace package `packages/client` from day one — the web SPA and the Electron shell are thin views over it (`@talkservo/client`, npm name TBC at publish). PoC: package builds in-repo, is NOT published. Beta: published, versioned against wire `v:1`. This is a same-language deliverable and does NOT alter D8's native binding matrix (C-ABI/C++/Py/UniFFI); Node bots consume the same package for free (pure TS). Facade shape: `new TalkServoClient({role}) → connect/requestFloor/on('floor'|'audio')`; wire types hand-written TS unions until core generates the schema.
**Date**: 2026-09-28 | **Limits**: PoC ships no npm artifact; naming collision check owed at publish (crates.io `talkservo` already taken by a placeholder — npm check required too). Option A (glue inside web/src) was rejected to avoid a Beta re-packaging refactor; cost accepted: +0.5 d in the web plan.
**Source**: user ruling (option B) 2026-09-28; precedent livekit-client/mediasoup-client (research/ui + media dossiers).

## D15: Native client media engine = multi-backend (MediaServo D139-140 isomorph): webrtc-sys primary, webrtc-rs secondary, stub test (2026-09-28)

**Decision**: User ruling, closing OQ-9: the native client (talkservo-client) media engine adopts the sister project's multi-backend compile-time-dispatch design — `backend-webrtc-sys` (prebuilt libwebrtc FFI, LiveKit rust-sdks crate as default candidate, MediaServo vendored fork as fallback; primary for production mobile), `backend-webrtc-rs` (pure-Rust secondary: desktop-Linux/CI builds, D3's engine becomes a live client backend here), `backend-stub` (tests). `#[cfg(feature)]` static dispatch, no runtime trait-object cost; compile_error on zero/dual backends (same discipline as server features). Supersedes the earlier OQ-9 phase order (platform-native-c first): rationale — mediasoup client protocol would otherwise be hand-written twice (Kotlin+Swift), while (b) writes it once in Rust behind the single D8 UniFFI facade; server-side transmission gating (D13) keeps the required client media surface minimal (transport/produce/consume/play only), which fits raw webrtc-sys.
**Date**: 2026-09-28 | **Amends**: OQ-9 (closed); D3 partially revived (webrtc-rs now a planned client backend, not just "candidate").
**Limits**: platform audio-session handling (Android AudioFocus / iOS AVAudioSession) stays native per-platform (~thin shell duty, not engine scope); webrtc-sys source (LiveKit upstream vs sister fork) verified at Beta kickoff; build matrix (libwebrtc prebuilts × platform/ABI in CI) is the accepted one-time cost.
**Source**: user ruling 2026-09-28; MediaServo D137/D139-140 verified this session; analysis thread in doc-review session (mediasoup-protocol-once argument).

## D16: Round-2 hardening package — queue purge, peer deltas, wire v/caps, dual-gate transmission, room TTL (2026-09-28)

**Decision**: Round-2 review (19 unique findings) design-level package: (1) `Leave/peer_left` purges queue entries with position rebroadcast — no ghost grants; (2) `PeerJoined{info,gen}`/`PeerLeft{peer,gen}` deltas, PeerList full-replace carries gen; (3) wire `v` field on Join echoed in Welcome, undeserializable frames → `Error{BadVersion}`+close; (4) caps `MAX_PEERS=10/MAX_QUEUE=8/MAX_FRAME_BYTES=64KiB` (config, overflow → Denied/413); (5) dual-gate transmission: client disables track off-floor AND server Producer::pause stays authoritative — acceptance #12 met by the pair (review H-2); (6) `ROOM_IDLE_TTL_S=600` reap destroys Router; (7) SDK contract: gen-discard `<` seen, strictly ordered events, `micTruth = enabled && !serverPaused`, auth-expired stops backoff; (8) `generation` seeds random high offset at boot (E7 guard).
**Date**: 2026-09-28 | **Amends**: D12/D13 (composition, no reversals). platform audio-session + PIPL/compliance remain product-domain backlog (CM-8), not designed here.
**Source**: arch-review2-team 2026-09-28 (CM/S/H/R2 lanes), all fixes user-approved 2026-09-28.
## D17: Acceptance range = LAN (same-subnet) — public-VPS slice deferred; coturn self-hosted, impairment media-only single-leg (2026-09-30)

**Decision**: User ruling: near-term target is a private/local-area network, no public host exists. The remaining plan-4 slice re-scopes accordingly — (1) deployment target = LAN bind (`TS_BIND` LAN IP + `TS_ANNOUNCED_IP=<LAN IP>`, Caddy `tls internal` for the secure-context requirement: browsers deny getUserMedia on non-localhost plain HTTP); (2) acceptance #9 ICE classes run against self-hosted coturn ON the LAN (srflx via local STUN, relay via local TURN — the candidate PATHS are exercised even though srflx address ≈ host address without a NAT); (3) acceptance #8 impairment moves from loopback-netem (two-leg cascade, PIT-8) to media-only single-leg: RTC ports into a mask-aligned range (40960-41215) + `tc filter` matching UDP source port only — upstream and TCP/WS planes stay clean; (4) real-device smoke is optional follow-on, simulation-first. The public-deployment design (Let's Encrypt, 4G budget line, WAN TURN posture) is NOT deleted — deferred to Alpha; the announced-IP switch and TURN-credential issuing are already built for exactly that later use.
**Date**: 2026-09-30 | **Amends**: plan-4 T4/T5 (VPS slice → LAN slice), acceptance #9/#8 environment definitions.
**Source**: user ruling 2026-09-30 ("目前无公网，仅局域网，初期目标是局域专网" + "同一个局域网，先在本机模拟" — chat-domain, artifacts per C1 remain English). <!-- c1:allow-zh -->
## D18: Admin minimal surface pulled forward to PoC-tail — Monitor + Tokens over out-of-band REST, no user store (2026-10-08)

**Decision**: LAN deployments need non-shell token issuance + a live floor view, so the admin surface ships at PoC-tail in minimal form: server `GET /admin/api/overview` + `GET /admin/api/rooms` + `POST /admin/api/tokens` (out-of-band REST — no SignalingMessage additions); auth = admin JWT with `aud:"admin"` under a physically separate `TS_ADMIN_SECRET` (the MediaServo D196 dual-token rule adopted verbatim; bootstrap via scripts/issue-admin-token.sh); room state read through a read-only actor command `RoomCommand::AdminInspect(oneshot)` (no shared-state restructure, no media-plane change). Web = the same SPA gaining an `/a` route (Monitor with 3s polling, Tokens form emitting copy-ready field/dispatch join links). Fail-closed: unset TS_ADMIN_SECRET disables the admin route group (503).
**Date**: 2026-10-08 | **Amends**: D9 (which deferred "admin console surfaces" wholesale to Alpha — this pulls forward exactly the two PoC-candidate surfaces its dossier flagged; everything else stays Alpha).
**Explicitly NOT pulled forward**: user store + forced first-run password change, audit/session persistence, keys/quota/limits/drain UI, WS AdminEvent push (polling v1; endpoints shaped to upgrade without breaking), per-room settings editor — all remain per admin-ui-patterns dossier §3 Alpha column.
**Source**: 2026-10-08 team research session (frozen dossiers re-read + Telegram/feature-matrix supplement); user confirmed design §1–§4 and the wireframe/architecture set; spec in docs/modules/08 §Admin surface, plan docs/plans/2026-10-08-plan5-admin-surface.md.
## D19: Ship tooling — oxmgr process supervisor + server facade + YAML instance config (2026-10-08)

**Decision**: Adopt the sister project's proven delivery model, scoped to TalkServo: (1) **oxmgr** (upstream Rust process manager, musl release binaries) vendored under `3rdparty/oxmgr/` as sha256-verified archives (scheme-Y: git keeps tarballs, cache unpacks, version-in-dir invalidation), injected into PATH by the outer shell, packaged into artifacts; (2) instance lifecycle as **facade subcommands inside talkservo-server** (`init/start/stop/restart/status/logs/monit/ps <dir>`, `run`/no-args = today's guarded direct start — systemd/compose chains untouched): `init` renders an instance dir (etc/server.yaml + identity secrets 0600 idempotent + run/oxfile.toml declaring the process), oxmgr does the actual supervision; (3) **YAML config layer** in the server (user readability ruling over TOML/env): precedence process-env > server.yaml > defaults, keeping systemd EnvironmentFile/compose environment authoritative when set; (4) outer shell `talkservo.sh` (bash, msrtc.sh lineage): build(stub|live lanes)/package(versioned tar, --strip-components contract)/deploy(--prefix idempotent assembly, never builds)/up|down(compose pass-through)/run(dev foreground)/doctor; absent artifacts (client/sdk/desktop) are exit-2 pointer stubs per D15/D8/D10.
**Date**: 2026-10-08 | **Amends**: none; complements D11 (Linux-native posture — oxmgr is a Linux musl tool; no-systemd edge hosts are exactly the D10/Electron-era scenario the facade serves), D16/D18 surfaces untouched (this is out-of-band tooling).

**Amendment B (2026-10-09, user-driven config-layout review)**: secret-duplication removed — `run/oxfile.toml` carries ZERO `[apps.env]` copies (run/ no longer a secret surface); `identity/env` = secrets ONLY (TS_JWT_SECRET/TS_ADMIN_SECRET/TURN_SECRET); `etc/server.yaml` is now the single operator file and also carries `bind:`. Resolution order becomes **process env > identity sidecar > yaml > defaults** (server sidecar-loads identity/env relative to its --config path; real-env snapshot keeps 12-factor authority; yaml outranks sidecar for shared keys, so legacy identity `TS_BIND` keeps working until the yaml says otherwise). Upstream oxmgr has no `env_file` (spec verified 2026-10-09) — tracking item to push for it; the sidecar is the local equivalent. Facade default instance dir also became the bin sibling when its `run/oxfile.toml` exists (deploy-layout `start` works from any cwd; `.server/` fallback kept).
**Limits**: single-server cluster (no host fleet) — the facade stays one binary + one optional caddy leg; remote SSH deploy explicitly deferred (tar-carry suffices for LAN); YAML parser dependency to be source-verified at T1 (serde_yaml EOL).
**Source**: ms_rtc survey (msrtc.sh 302 lines + mediaservo_cli.py 24-cmd + 3rdparty/oxmgr recipe), 5-point one-by-one adjudication 2026-10-08 (plan-6 record), AGENTS no-scaffolding anti-pattern honored by stub choice.
## D20: Default cluster shape = server + caddy TLS front (sister parity); --no-web for single-process (2026-10-08)

**Decision**: `talkservo-server init` defaults to the FULL two-process cluster (matching MediaServo's oxfile shape): auto self-signed LAN cert (openssl, SAN = detected LAN IP + talkservo.test), ENABLED Caddyfile (`https://:8443` → reverse_proxy 127.0.0.1:8080, `auto_https disable_redirects` — the auto :80 hop is permission-denied unprivileged, measured), caddy joins run/oxfile.toml, backend bind narrows to loopback (devices enter via :8443 only — one port, TLS-satisfying the browser secure-context rule for LAN getUserMedia). Degradation is loud and never silent: no caddy binary OR no openssl ⇒ server-only cluster + WARN + backend stays 0.0.0.0:8080 (plain http stays LAN-usable in that fallback). `start/restart --no-web` = apply --only talkservo-server (dev form; embedded rust-embed web still serves everything on 8080 directly). `run` and systemd/compose tracks untouched. The web SPA is embedded in the talkservo-server binary (live lane), so caddy is the TLS/entry front, NOT a static server — this is the deliberate divergence from MediaServo's architecture (their SPA is loose files caddy must serve; ours is baked).
**Date**: 2026-10-08 | **Amends**: D19 (cluster default flips from "caddy only when certs manually staged" to "certs auto-generated, caddy is the default"); no wire/media-plane change.
**Limits**: stub-lane binaries have no embedded web — SPA routes 404 there BY DESIGN (verified: /healthz still fine through the front); :8443 default front port (SUPERSEDED by B2 2026-10-09: server.yaml front_port is the source of truth; the Caddyfile is a generated artifact rewritten on init/deploy); single shared 8443 per host (multi-instance = per-instance ports, future).
**Source**: user directive 2026-10-08 (对标 mediaservo：默认 server+web 两进程、单 server 加参数) <!-- c1:allow-zh -->; sibling shape from mediaservo-server templates.rs (caddy entry + --only whitelist + degrade-warn); live smoke: 2 running + TLS→backend healthz + restart --no-web single-app.

## D22: Admin password login — PBKDF2 hash in server.yaml, server-minted short-lived JWT (2026-10-09)

**Decision**: user rejected the script-mint-and-paste operator UX (2026-10-09: "admin JWT is inconvenient — why not a password from the config file? I want to follow mediaservo"). Landed the sister shape (`POST /api/auth/login` -> JWT; ms_rtc PLAN.md:242 evidence), hardened two notches: the stored credential is a **PBKDF2-HMAC-SHA1 hash** (600k iters, 16B urandom salt, `pbkdf2$iters$hexsalt$hexdk`, constant-time verify; zero new deps — hmac/sha1 already workspace deps from turn.rs) in `server.yaml admin_password_hash:`, not a plaintext password; the minted session is a **short-lived HS256 admin JWT** (`aud:"admin"`, TTL `TS_ADMIN_LOGIN_TTL_S` default 12h) signed with the existing `TS_ADMIN_SECRET` — audience isolation and the guard stay byte-identical. Set via `printf '%s' PW | talkservo-server hash-password` (stdin-only, sister PIT-171 honored); login is rate-limited (60/min shared bucket) ahead of verification; 503 fail-closed when hash unset preserves today's behavior; `scripts/issue-admin-token.sh` kept for bootstrap/rotation.
**Revision 3 (same day)**: user overruled two assistant positions with sister evidence — (1) "deploy must not mint credentials" was WRONG vs ms_rtc T16 (init generates accounts.yaml + secret bootstrap, PIT-160 idempotent); init now bootstraps `admin` with a plaintext `password:` row (sister ops shape: hand-editable registry, hot via read-per-login, API upsert writes hash rows and clears plaintext). (2) the instance lives at `out/live` (repo default), NOT relocated out-of-repo — the migration is reverted to default usage; out-of-repo `--prefix` stays documented as the optional defense.
**Date**: 2026-10-09 | **Amends**: refines D18 (admin surface — script-only mint becomes one of two mints); consumes the modules/08 line "Admin JWT … surface built at Alpha" early.
**Limits**: single admin identity (sub="admin") — accounts/user store stays Alpha (D18 non-goals); no logout endpoint (client clears sessionStorage); PBKDF2-sha1 chosen over argon2 for zero-dep, revisit if a hash crate lands anyway.
**Revision 2 (same day)**: first landing was PASSWORD-ONLY (single implicit "admin") — user rejected (「我想要的是 user/pwd 方式」). <!-- c1:allow-zh --> Now sister-shaped: `POST /admin/api/login {user,password}` against `etc/accounts.yaml` (program-maintained registry `[{user,hash,role}]`, atomic tmp+rename write, read-per-login no cache), `GET/POST/DELETE /admin/api/accounts` (list never carries hashes; upsert=password change; hot — new accounts log in without restart), UI Gate = Username+Password (JWT paste removed; script mint stays at CLI for bootstrap), Sider gains **Accounts** page. While the file is absent the implicit `admin` row (server.yaml `admin_password_hash:`) authenticates; first upsert CARRIES it into the file (no stranding). `TS_ACCOUNTS_FILE` env overrides the derived path (e2e/bare-env runs). Role stored but single-valued ("admin") — multi-role (sister viewer/operator/dispatcher) stays Alpha with the user store.
**Verification**: kdf RFC6070 KAT x3 + malformed fail-closed (35 lib tests); accounts roundtrip/implicit-carry/corruption-tolerance unit tests; integration 5/5 stub lane (CRUD full chain incl hot-login, weak-pw 400, user-charset 400, double-delete 404, unauthed 401, aud-isolation); vitest 6/6 (two-field gate); e2e admin.spec 1 passed (bootstrap-token -> create account -> UI login -> issue -> click-through -> wrong-pw + user-token bounces); live: ops+admin registered via API on out/live, both login 200, wrong 401.
**Source**: user question 2026-10-09 -> option adjudication (question tool) -> 「执行 sister 式密码登录」; ms_rtc docs archaeology (accounts.yaml, /api/auth/login, PIT-171 credentials lesson). <!-- c1:allow-zh -->

## D21: Config single source — server.yaml is the ONLY operator file; Caddyfile/oxfile demoted to generated artifacts (2026-10-09)

**Decision**: user challenged the split config surface ("主流项目都是集中在一个文件"); resolved in two landed steps. **(B) secret-duplication removal**: `run/oxfile.toml` carries zero `[apps.env]` copies; `identity/env` = secrets-only (0600) loaded by the server as a sidecar of `--config`; precedence **process env > identity sidecar > yaml > defaults** (real-env snapshot; empty values never clobber); `bind` moved into yaml. **(B2) front port absorbed**: `front_port:` in server.yaml (default 8443), `render_caddyfile(port, backend)` parameterized, `etc/Caddyfile` demoted from operator-preserved asset to GENERATED artifact rewritten on init/deploy; `public_base` auto-derives `https://<lan-ip>:<front_port>` on fresh init. Result: every port/IP/tuning edit happens in exactly one file. Upstream oxmgr 0.5.0 has no `env_file` (spec verified 2026-10-09) — sidecar is the local equivalent, tracking item open to adopt upstream if added. <!-- c1:allow-zh -->
**Date**: 2026-10-09 | **Amends**: D19 (config precedence + literal secret rendering — see its Amendment B), D20 (its limit ":8443 fixed default port, edit the Caddyfile" is SUPERSEDED).
**Limits**: front port change flow = edit yaml -> deploy (re-renders Caddyfile) -> restart (no hot-swap); NO 301 for old SPA routes (PoC has no installed link base — C3 renames went full-word instead); multi-instance global-name collision stays PIT-12 (single-instance contract).
**Source**: user ruling chain 2026-10-09 「主流项目都是放在一个 yaml…分析」->「b方案，配置文件有哪些？端口、ip 等等如何配置？」->「当前 server 和 web 配置是分散的…说人话」->「执行」; verification = 28 lib tests + live closed loop (`https://192.168.2.127:8443/field/demo?jwt=…` issued via /admin). <!-- c1:allow-zh -->

## D23: Alpha wave 1 = trust & control — room authority chain, audit log, role tiers, intervention kit (2026-10-19)

**Decision**: four sub-decisions adjudicated one by one (user-adjudicated walkthrough protocol 2026-10-19, "B/A/A/A/B"):
1. **Room authority = token-carried moderator chain (option B)**: JWT claims gain `mod: [room]` list; server-side authorization check ADDED for command messages — today ModeChange/MuteSet drop the actor (`let _ = actor;` floor.rs) so ANY field token can flip modes (verified gap, never formalized). Per-room authority (creator/moderator per Zello model, competitor lesson 3) chosen over global-dispatch superuser; claims-shape change ripples to SDK types/auth/tests.
2. **Audit = JSONL append files (option A)**: `out/live/logs/audit.jsonl` (rotating), one line per floor/admin/auth event (grant/taken/idle/mode_change/mute/login-fail/join/leave/token-issue); admin REST `/admin/api/audit` + Logs tab; zero new deps; SQLite deferred until a real query need exists (format migrates losslessly).
3. **Account tiers = admin + operator (option A)**: `role` field activated — admin (accounts/rooms/audit config) vs operator (issue tokens, monitor, act as room moderator); viewer tier deferred (YAGNI, no stated consumer); changes concentrate in admin guard.
4. **Intervention = full kit of three (option A, user upgraded from recommended B)**: kick (DELETE /admin/api/rooms/{room}/peers/{peer}, reusing the Leave cleanup path) + forced floor release + per-peer detail panel (transport stats via AdminInspect extension); detail panel accepted despite SFU-stats chain cost.
5. **Order = audit skeleton FIRST (option B)**: audit writer lands first so every later step (authz denials, 403s, kicks) auto-leaves evidence — no bare-window where power acts are unlogged; intervention kit lands last with gate+audit in place.
**Date**: 2026-10-19 | **Amends**: D18 (audit persistence + roles + user-store items pulled forward from Alpha non-goals — this is the Alpha wave-1 scope ruling); consumes the actor-authorization gap discovered 2026-10-19 (floor.rs `let _ = actor;`).
**UI annex (same day, three rulings B/A/A/A/B walkthrough + UI walkthrough)**: admin console = five tabs `Monitor | Rooms | Tokens | Accounts | Logs`, admin.tsx split into `pages/admin/` (index/monitor/rooms/tokens/accounts/logs, each <=150 lines); intervention = Monitor row-expandable peer list, kick/force-release behind Popconfirm, peer detail Drawer (AdminInspect per-peer extension); role tiering is layout-visible (Accounts tab admin-only, server 403 is the real gate); operator-audible Listen button = wave-2 (requires wire AdminListen/Role + SFU admin consume path — deferred WITH AdminEvent WS push as the wave-2 protocol batch).
**Team-review annex (same day, U1-U3 walkthrough)**: U1 = dead code + tautology tests fixed
IMMEDIATELY, not plan-gated (fake acceptance evidence does not wait for a plan; media.spec #12
`>= -1` tautology -> real micTruth().serverPaused assertion, dead gain Slider/PREEMPT_PRIORITY/
AudioCues-header honesty, floor.spec dead line, talk.spec cross-client `Speakers (2)` + `mode:
open` non-actor assertions per PIT-21 rule, admin.test 401-bounce, playwright config parity
retries:0/forbidOnly/trace); U2 = all three borrow items absorbed into wave-1 (Discord audit
5-tuple -> S1 schema; Jitsi kick contract -> S6; Mumble suppress deny!=mute -> S7.5);
U3 = behavioral-contract tests fixed now, UI-position tests reborn with the S7 rebuild (each
test line survives refactor day).
**Plan-review annex (same day, R1-R4 walkthrough)**: R1 = registry takes the clean name
`/admin/api/rooms[/{room}]`, live poll renames to `rooms/live` (six call sites; one-time while
no external consumers — D14 unpublished); R2 = kick reason is AUDIT-only, room broadcast reuses
PeerLeft with an additive `left:removed` kind marker (no reason/actor over the wire — privacy
+ awareness, old clients valid); R3 = deny!=mute copy is its own slice S7.5 (separate commit,
droppable without touching the S7 mainline); R4 = gates gain a 4-identity x 3-endpoint
permission-matrix truth table, audit fail-visible fault injection (write failure MUST count +
tracing::error, never panic, never silently drop — PIT-13 lesson), legacy-token compat test
(mod_rooms-less token = empty set); static grep gate REJECTED (compile-time struct already
pins the schema). Docs-gap ruling same day: modules/02 claims-debt payoff + modules/01 Rule 10
IN as sync obligations; modules/06/03 + architecture-acceptance rows NOT adopted (explicit,
ledgered here).
Full plan: docs/plans/2026-10-19-alpha-wave1-trust-control.md (18 adjudications, all closed).

**Limits**: single-room PoC heritage — multi-room authority shapes (mod lists per token) need the claims change verified against SDK wire mirror; FEC/netem acceptance #8 still outstanding; these four items are Alpha wave 1, NOT Beta scope.
**Source**: brainstorming full-landscape review (2026-10-19): whitepaper §11 roadmap Alpha items already proven by PoC core (R1-R9) → strategic re-aim at "trust & control" dimension; competitor lessons 2/3 (audit+priority are what buyers pay; flat rooms are toy-grade); oss-voice-infra precedents (Mumble ACL / LiveKit grants / Janus admin / Jitsi affiliation — all server-enforced, all ≥2 tiers); user rulings in 5-adjudication walkthrough (plain-text protocol per adjudication-walkthrough skill).
