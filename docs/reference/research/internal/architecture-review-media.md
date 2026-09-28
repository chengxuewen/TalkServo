# Architecture Review — Media Plane (D6/mediasoup) vs Production WebRTC Practice

> Review dossier 2026-09-28 (arch-review-team, media-reviewer). New dated file per C2 — annotates, never back-writes, frozen dossiers.
> Scope reviewed: modules/02 §W, modules/03, modules/04, modules/05 (E4/E6/E10), whitepaper §media, reference/research/media/*. Method: design claims tested against mediasoup official docs + the `mediasoup` Rust crate source (versatica/mediasoup `rust/`), RFC 6716/7587 full text, LiveKit docs, talktome README, mumble.info.

## Findings

| # | Severity | Area | Verdict |
|---|----------|------|---------|
| M-1 | HIGH | "relay-only SRTP" framing | contradicted by mediasoup architecture — server terminates and re-originates DTLS-SRTP |
| M-2 | HIGH | M1 always-publish + DTX off + 2ch | always-hot decryptable uplink; mobile data/battery cost; frozen rationale is false |
| M-3 | HIGH | E-matrix runtime media death | gap — floor lock-up when holder's uplink dies with WS alive |
| M-4 | MED | apply_floor mechanism | modules/03 vs 04 vs 02 spec two different mechanisms; pick pause/resume |
| M-5 | MED | one worker per process | OK for PoC; ~50-transport cap has no upstream basis; plan round-robin now |
| M-6 | MED | TCP fallback (enableTcp) | works but wrong priority order; ICE-Lite has no restart |
| M-7 | LOW | announcedAddress single-stack | fine for PoC; multi-homed learning from talktome noted |
| M-8 | LOW | no BWE story | acceptable audio-only; revisit at Alpha video |
| M-9 | LOW | frozen dossier cites "RFC 8215" for Opus | wrong RFC — correction record |
| M-10 | LOW | codec pinned 2ch | mono is the speech standard; halves uplink cost |
| M-11 | LOW | crate pinned 0.24 | latest 0.28.1 (official, in-tree); pin exact + upgrade plan |

### M-1 (HIGH) — "SFU relays, no crypto touch" is not what mediasoup does
Whitepaper "SFU -- audio relay -->" and the frozen pipeline diagram (audio-processing-stack.md §4: "RTP parse → relay decision (no decode)") predate D6. mediasoup WebRtcTransport terminates DTLS-SRTP **per transport inside the C++ worker** (DtlsRole `auto`; ICE-Lite → worker is always controlled/DTLS-client role per mediasoup-client docs); each consumer re-encrypts with its own keys. The server therefore holds **plaintext audio of every joined peer**, not just the holder (see M-2). modules/06 "SRTP mandatory (mediasoup default); no plaintext path" is correct on the wire but silent on server-side decryption capability, and mediasoup has **no E2EE path** (SFrame not supported; str0m-ecosystem comparison). Fix: one trust statement in modules/06 ("worker decrypts all SRTP; server operator is trusted; E2EE out of scope until Alpha — track as OQ") + this file as the C2 annotation of the frozen dossier.

### M-2 (HIGH) — always-hot mic to a decrypting server; DTX-off rationale is false
M1: "publish always, relay by grant" — grant gates **relay, not transmission**. The frozen dossier's DTX-off rationale ("floor state already gates transmission", audio-processing-stack §1) is false under M1: every peer streams continuously. Contrast Mumble: transmission is client-gated (PTT/VAD default per mumble.info audio-settings). Costs: Opus FB speech sweet spot 28–40 kbps mono (RFC 6716 §2.1.1); with 2ch + FEC + DTX off ≈ 40–64 kbps continuous uplink per idle peer on mobile data, plus battery, plus the privacy posture of M-1. The zero-renegotiation intent is right — keep the transport and producer alive, but: (a) client disables the sender track (or server calls `Producer::pause()` — verified in crate) when not holding, resume on grant; (b) revisit `DTX on` for idle uplink. Severity honest: this is a design-completeness gap, not a bug — dispatch users may accept "server can hear me", but it must be *stated*, not implied.

### M-3 (HIGH) — no runtime media-failure row: floor lock-up scenario
E4/E5 cover creation failures; E6 covers worker death. Missing: holder's **uplink dies while WS lives** (4G handover, app backgrounded). mediasoup does not auto-close producers on RTCP BYE or RTP silence. Available hooks (all verified in crate/docs): `iceConsentTimeout` default 30 s → transport auto-close (RFC 7675); `icestatechange('disconnected')` — docs say *not recoverable, application should close*; `dtlsstatechange('closed')` on CloseNotify. Wire: transport `close` → `MediaFailed{peer}` → floor auto-release (same path as E1). Residual gap: consent covers STUN/RTP-any traffic — a peer still sending RTCP/STUN but silent RTP is *not* detected; PoC can add a "no RTP for N s" watchdog via producer score/`rtp` listener, or accept 30 s consent as the backstop. Without any of this, a silent holder keeps the floor indefinitely — worst failure mode for dispatch.

### M-4 (MED) — pause/resume vs create/close: the docs specify both
modules/04 M1: "audibility = Router pause/resume state, zero renegotiation per turn". modules/03: `apply_floor` "idempotent: **diff consumers** to state" (reads as object diff). modules/02 P-sequence: "grant → ConsumeOk per listener" (reads as create-at-grant). The crate exposes `Consumer::pause()/resume()` and `Producer::pause()` (verified in `rust/src/router/consumer.rs`/`producer.rs`), so pause/resume is implementable. Recommend: create consumers once per (listener×producer) at Produce/join, `apply_floor` diffs **pause state**, grant = resume + one `ConsumeOk`-equivalent push — this is what makes the ≤300 ms P-budget credible (create/close costs a worker request + client bind per turn, N times). N² pre-created consumers is trivial at PoC fan-out (small dispatch rooms; cap M-5). Re-word modules/03/02 to say so.

### M-5 (MED) — worker topology
mediasoup's model: one worker = one C++ subprocess pinned to one CPU core; scale = spawn per-core and round-robin Routers (crate ships `worker_manager` for exactly this). PoC "one managed binary supervising its worker" (modules/03) is fine. The E10 "~50 transports" cap has **no upstream basis** — mediasoup documents no per-worker transport cap; audio-only forwarding is CPU-bound and handles far more per worker. Keep 50 as a config-driven guardrail (fine for PoC) but (a) say "arbitrary guardrail, measure at acceptance" instead of implying capacity truth, and (b) put router→worker placement behind the supervisor interface now — retrofitting round-robin + PipeTransport (OQ-4) into a single-worker-hardcoded supervisor at Alpha is the expensive version.

### M-6 (MED) — TCP fallback reality on 4G
`enable_tcp` (crate default `false`, confirmed) advertises ICE-TCP candidates on the **same RTP port range** → firewall must open TCP 40000–40100 too; talktome ships exactly this (both `/udp` and `/tcp` publishes of 40000–49999 — production precedent). Two caveats the design doesn't state: (1) TCP real-time audio over lossy 4G suffers head-of-line blocking → latency spikes; direct ICE-TCP on high ports is often blocked by the same middleboxes that block UDP — the robust path is **TURN-TLS/443 via coturn**, so candidate priority should be UDP → relay(TLS) → ICE-TCP, not "enableTcp = the 4G fallback" (OQ-8's measure-don't-assume is the right instinct). (2) mediasoup is ICE-Lite: **no ICE restart** — a dropped connection means transport close → fresh transports via the W-style re-handshake; M-3's row must cover this, not just creation-time E5.

### M-7 (LOW) — announcedAddress
`listenInfos/announcedAddress` required when binding 0.0.0.0 (Docker) — design has it; IPv4-only PoC OK (OQ-7). talktome production learning: multi-homed/LAN+NAT hosts advertise **all usable adapters** (or preferred-adapter/manual mode); mediasoup supports multiple `listenInfos` — document at Alpha, don't hardcode single-address in the config schema.

### M-8 (LOW) — BWE
No server-side bandwidth-probing story in the design. Audio-only PoC: browser congestion control adapts Opus bitrate client-side; `initialAvailableOutgoingBitrate` (600 kbps default) only matters for REMB/Transport-CC consumers (video layers). Acceptable omission; reopen with Alpha video scope.

### M-9 (LOW) — RFC citation bug in frozen dossier
audio-processing-stack.md cites "RFC 8215 (formerly 6716)" as the Opus FEC spec throughout. RFC 8215 is **"Local-Use IPv4/IPv6 Translation Prefix"** (fetched, confirmed) — not Opus, and 6716 was never superseded. Correct authorities: codec + in-band FEC + DTX = **RFC 6716 §2.1.7 / §2.1.9**; RTP/SDP mapping of `useinbandfec`/`usedtx` fmtp = **RFC 7587 §3.1.3/§4.1**. The technical content of the dossier is otherwise consistent with 6716/7587; live docs (modules/04) don't cite the id, so nothing propagated. Dossier stays frozen; this file is the correction record (PIT-3 discipline).

### M-10 (LOW) — 2ch pinned for speech
Router mediaCodecs pin 48 kHz **2ch**. RFC 7587 defaults `stereo`/`sprop-stereo` to 0 (mono); speech sweet spots are mono figures (6716 §2.1.1); stereo doubles uplink bitrate and receiver processing with zero dispatch benefit. Stereo only earns its cost for program-audio feeds (talktome precedent) — out of PoC scope. Recommend mono; keep 48 kHz FB.

### M-11 (LOW) — crate version drift
D6/modules/03 pin `mediasoup` 0.24; crates.io latest **0.28.1** (2026-09-16, ISC, official in-tree `rust/` in versatica/mediasoup — maintenance signal is good, better than expected for a "Rust binding"). Minor bumps rotate the worker protocol/SRTP surface. Keep Cargo.lock-pinned exact 0.24.x for PoC; schedule an Alpha upgrade with the same e2e gate as E6.

## What the design gets right (verified against sources)
- M1 zero-renegotiation coupling maps 1:1 onto real crate API (`Consumer::pause/resume`, `Producer::pause`) — the D6 choice was sound.
- Short-TTL HMAC TURN creds in `Welcome` — matches talktome (`TURN_URLS` env, no static page keys) and LiveKit practice.
- netem FEC A/B with a quantitative `concealmentEvents` pass line — stronger acceptance discipline than any precedent found (Mumble/LiveKit ship FEC without published A/B gates).
- Router-as-dumb-forwarder + arbitration in core — exactly mediasoup's intended seam (design doc; oss-voice-infrastructure §3 lesson).
- W-sequence (worker restart, floor state untouched) — correct: worker death loses transports only; DTLS/ICE fresh-objects + client re-handshake matches mediasoup-client lifecycle.

## Sources
- Local: docs/architecture.md; docs/modules/{02,03,04,05,06}; docs/whitepaper.md; docs/reference/research/media/{audio-processing-stack,media-stack-alternatives,oss-voice-infrastructure}.md.
- mediasoup.org v3: /documentation/v3/mediasoup/design/; /mediasoup/api/ (Worker "single CPU core"; WebRtcTransportOptions enableTcp/iceConsentTimeout=30/initialAvailableOutgoingBitrate=600000; dtlsstatechange/icestatechange semantics; DtlsRole/ICE-Lite); /mediasoup-client/api/ (DTLS role selection).
- `mediasoup` Rust crate: crates.io API (0.28.1 latest, 0.24.3 line, repo versatica/mediasoup); docs.rs (worker_manager, Consumer/WebRtcTransport); GitHub contents API `rust/src/router/{consumer,producer,webrtc_transport}.rs` (pause/resume verified; enable_tcp default false; ice_consent_timeout default 30; ice_state_change event).
- IETF: RFC 6716 §2.1.1/2.1.7/2.1.9 (bitrate sweet spots, FEC, DTX); RFC 7587 §3.1.3/§4.1 (DTX, stereo/useinbandfec fmtp); RFC 8215 fetched — confirmed IPv4/IPv6 translation prefix (M-9).
- LiveKit docs: /transport/media/{advanced,publish,subscribe}.md (dtx/red/audioBitrate publish options; autoSubscribe).
- talktome (thepoison606/talktome) README via api.github.com: RTC port range TCP+UDP publishing, PUBLIC_IP/announced modes, TURN env config.
- mumble.info /documentation/user/audio-settings/: client-side PTT/VAD transmission gating; 20 ms frame guidance; low-delay mode.
