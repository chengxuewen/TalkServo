# Media Stack Alternatives — WebRTC Engine, FEC, 3A

> **Status: FROZEN dossier (C2).** Point-in-time research; conclusions may be superseded (e.g. D6 engine change) — live truth is docs/architecture.md + docs/modules/. Do not back-write; new findings get new dated files.

> Research snapshot: 2026-09-28. Supporting doc for [whitepaper.md](../../../whitepaper.md) §7 and [../architecture.md](../../../architecture.md) §4. Facts verified against repo state on snapshot day; drift expected.

## 1. WebRTC engine comparison

| Engine | Lang | Maintenance @2026-09-28 | SFU asset | Built-in 3A/NetEQ | Audio FEC path | Fit for TalkServo |
|--------|------|--------------------------|-----------|-------------------|----------------|-------------------|
| **webrtc-rs/webrtc** | Rust | active (pushed 2026-09-27, 5.1k stars, 5 open issues) | **no complete SFU example** — org ships a sans-IO `sfu` building-block crate (active, pushed 2026-09-13); examples mirror Pion (`broadcast`, `rtp-forwarder`, `simulcast`); `pion/ion-sfu` (Go; moved to ionorg, inactive since 2023-07-21) is the architecture reference | no | Opus in-band FEC + `fec=true` decode must be wired manually | **selected (D3)**: pure-Rust, language-aligned; we accept hand-building loss handling |
| libwebrtc | C++ | Google upstream | full stack incl. NetEQ/3A | yes | built-in | rejected — build weight, FFI complexity; sister project MediaServo's libwebrtc boundary pain is cautionary evidence |
| mediasoup | TS control + C++ worker | very active; proven in the wild by talktome (123 stars, pushed snapshot day) | production SFU incl. Router/Transport API | relies on browser endpoints; server relays | relay preserves end-to-end FEC | **fallback option**: battle-tested intercom precedent; FFI/worker complexity we deliberately avoided in PoC |
| libdatachannel | C++ | active | network layer only (no SFU logic) | no | fully DIY | C/C++-ecosystem alternative; no advantage for a Rust core |
| pion/webrtc | Go | very active (16.8k stars, pushed 2026-09-26) | no SFU example in main repo; `ion-sfu` inactive since 2023 (repo not archived) | no | manual | ecosystem anchor; Gryt proves the shape but Go breaks Rust-core consistency |
| browser RTCPeerConnection | — | W3C stacks | n/a (client) | yes (client-side) | yes (client-side) | client side of the PoC regardless of server engine |

Decision consequence: **the server media layer is ours to compose** — from webrtc-rs primitives (`broadcast`/`rtp-forwarder` patterns), with the floor-grant/revoke set driving which SSRC relays to which peers. OQ-1 in architecture.md stays open on shape (trim-from-broadcast vs custom router), but the "reuse official SFU crate" assumption from the planning discussion is **corrected: the org's `sfu` crate is a sans-IO building block, not a turnkey SFU**.

## 2. Opus in-band FEC (transport-agnostic audio recovery)

WebRTC audio loss resilience lives in the **codec**, not the transport:

```text
encoder: frame N packet = primary(N) + low-bitrate redundancy of N-1   (inband FEC, SILK)
network: packet loss
receiver: seq gap detected -> decode frame N with fec=true (uses embedded N-1 parity)
          no parity either -> PLC (packet-loss concealment) from decoder state
```

| Concern | Owner in webrtc-rs stack | Work for us |
|---------|--------------------------|-------------|
| FEC encoding | Opus encoder (`OPUS_SET_INBAND_FEC(1)`, VBR, typically 20 ms frames) | set at encoder init; browser client: SDP `usedtx`/opus attributes |
| Loss detection | RTP receive path | track seq/SSRC gaps |
| FEC re-decode | Opus decoder second pass `fec=true` | implement in receiver pipeline |
| PLC | Opus decoder | free with libopus |
| Relay integrity | SFU forwards RTP untouched | preserve FEC; do not transcode server-side |
| Jitter buffer | **missing** in webrtc-rs for our use | PoC: minimal adaptive buffer; grow toward NetEQ-lite in productization |

## 3. 3A (AEC + AGC + ANS) routes

| Route | Components | Pros | Cons | Status |
|-------|-----------|------|------|--------|
| **FFI wrapper (PoC, D4)** | `webrtc-audio-processing` crate (Google algorithm port) | production-proven; the same AEC that ships in Chrome-class stacks | C++ toolchain in build graph; Linux-first CI; feature-gate a passthrough stub for unsupported targets | **selected** |
| Pure-Rust | `aec3-rs`, `sonora` | no FFI; cross-platform parity | younger APIs; AEC3 perf unproven at our scale | productization evaluation |
| Composed | `nnnoiseless` (NS) + `decibri-aec` (AEC) + `dagc` (AGC) | swappable pieces, per-module licensing | pipeline glue and interactions are ours to validate | research note only |

Browser caveat: the web client gets 3A **free** from `getUserMedia` constraints (`echoCancellation`, `noiseSuppression`, `autoGainControl`); the 3A crate path matters for **native** clients (mobile/desktop SDK via UniFFI), where we own capture. PoC browser flow therefore validates signaling/floor without fully proving the 3A pipeline — schedule native 3A verification for Beta.

## 4. Codec & transport baseline (locked for PoC)

| Parameter | Value | Why |
|-----------|-------|-----|
| Codec | Opus, VoIP mode, 48 kHz, 20 ms, VBR, inband FEC on | WebRTC-native, FEC-ready, Mumble/talktome/Gryt precedent |
| Transport | SRTP over UDP via webrtc-rs (DTLS handshake) | no plaintext RTP fallback (architecture §7) |
| NAT | STUN + coturn TURN, ICE full negotiation even on loopback | MediaServo lesson: localhost WebRTC fails without explicit ICE |
| DTX | off in PoC | avoids comfort-noise complexity while floor state is the real gate; revisit for battery scenarios |

## 5. Open items

| # | Item | Resolve by |
|---|------|-----------|
| M-1 | webrtc-rs audio-only relay shape from `broadcast`/`rtp-forwarder` examples (OQ-1) | PoC stage 3 |
| M-2 | jitter-buffer size policy under floor churn (rapid grant/revoke) | PoC stage 3 manual QA |
| M-3 | `webrtc-audio-processing` crate version + C++ standard matrix | before Beta (native SDK) |
| M-4 | DTX/comfort-noise for mobile power | productization |
| M-5 | re-verify engine maintenance status before Alpha | Alpha kickoff |

## 6. GitHub saturation sweep findings (2026-09-28, see [github-sweep-webrtc.md](github-sweep-webrtc.md))

| Candidate | Verified status | Relevance |
|-----------|-----------------|-----------|
| `webrtc-rs/sfu` crate | **exists**, 82 stars, active — the sans-IO building block (not turnkey) | primary OQ-1 base |
| `algesten/str0m` | 627 stars, pushed snapshot day; sans-I/O; spawning oxpulse-sfu-kit/lumyx/str0m-e2ee | OQ-1 alternative base; MediaServo sister listed str0m as Phase 2+ too |
| `MixinNetwork/kraken` | 358 stars, Go, production audio-only (Opus) SFU | closest published architecture to our relay shape (language mismatch with D3) |
| `tonarino/webrtc-audio-processing` | 330 stars, Rust FFI — **the** D4 crate | D4 confirmed canonical |
| audio-sfu-server (0★), oxpulse-sfu-kit (3★) | pre-alpha, right shape | watch list only |

Net effect on OQ-1: three concrete bases now verified — compose-from-examples, `webrtc-rs/sfu` crate, or str0m. The option space is no longer speculative; decision stays open for PoC stage 3.
