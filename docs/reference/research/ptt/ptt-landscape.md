# PTT Open-Source Landscape — Verified Profiles & Benchmark

> **Status: FROZEN dossier (C2).** Point-in-time research; conclusions may be superseded (e.g. D6 engine change) — live truth is docs/architecture.md + docs/modules/. Do not back-write; new findings get new dated files.

> Research snapshot: 2026-09-28, via GitHub REST API (stars/last-push verified per repo).
> Purpose: competitive/reference material for TalkServo (see [whitepaper.md](../../../whitepaper.md), [architecture.md](../../../architecture.md)).
> Note: this archive corrects several claims from the initial planning discussion; corrections are flagged explicitly.

## 1. Summary matrix

| Project | Repo | Stars | Last push | Stack | Voice? | Media path | Floor/arbitration | Status read |
|---------|------|-------|-----------|-------|--------|-----------|-------------------|-------------|
| talkkonnect | talkkonnect/talkkonnect | 363 | 2026-09-02 | Go | yes (Mumble client) | Mumble UDP (server mixes/routes), multicast RTP & SIP bridges | Mumble server-mediated PTT; client does not arbitrate | **active**; hardware/interop angle |
| EVO-PTT | Theofilos-Chamalis/EVO-PTT | 118 | 2026-05-04 | Java (Android) + closed voice server | yes | proprietary server (Docker-isolated) | server-side; hardware PTT key injection (F22/F25) | **commercial showroom**; core not open |
| talktome | thepoison606/talktome | 123 | 2026-09-28 | Node.js + **mediasoup** + Socket.IO | yes | mediasoup SFU, WebRTC browser clients | "talk lock" server-mediated; admin UI | **most active in set** (pushed on snapshot day) |
| Gryt | Gryt-chat/gryt | 37 | 2026-09-25 | Go SFU (Pion-based) + web/desktop | yes | self-hosted WebRTC SFU | global PTT keybinds; server relay per channel | **active**; Discord-shaped product |
| Radio-Link | Radio-Link/Radio-link | 0 | 2025-11-07 | Flutter + WebRTC P2P + Socket.io + Postgres | yes | P2P mesh (TURN via Twilio/coturn); optional Janus | none — implicit "press to talk, be audible" | dormant prototype |
| Zello_Walk | RobCrack2023/Zello_Walk | 0 | 2025-11-16 | Node/Express + Socket.io PWA | yes | **Socket.io audio relay, not WebRTC** | none visible in server code | 19-file toy; "800-channel" claim **unverified** |
| OpenPTT | OpenPTT/OpenPTT | 355 | 2016-01-21 | Cordova + AngularJS | **no — text BBS client for ptt.cc** | n/a | n/a | stale decade; **not a voice peer** |

Ecosystem anchors:

| Anchor | Stars | Last push | SFU asset |
|--------|-------|-----------|-----------|
| pion/webrtc (Go) | 16,803 | 2026-09-26 | no dedicated SFU example in main repo; `broadcast`, `rtp-forwarder`, `simulcast`, `reflect`, `whip-whep` are the primitives; separate `pion/ion-sfu` project moved to `ionorg/ion-sfu`: not GitHub-archived but dead since 2023-07-21 |
| webrtc-rs/webrtc (Rust) | 5,154 | 2026-09-27 | mirrors Pion's example set (`broadcast`, `simulcast`, `rtp-forwarder`); no complete SFU example, though the org ships a sans-IO `sfu` building-block crate |

## 2. Corrections vs. initial planning discussion

| Claim (whitepaper v0.1 discussion) | Verified reality | Impact |
|-------------------------------------|------------------|--------|
| "OpenPTT — classic PoC voice platform" | Text BBS terminal app (ptt.cc), no voice at all | removed from peer set; kept as naming-history footnote |
| "Zello PWA clone, channels up to 800" | 0-star, 19-file Socket.io relay, in-memory, no persistence, no WebRTC | downgraded to "what not to build" evidence |
| "webrtc-rs: official SFU crate reusable" | no complete SFU example; sans-IO `sfu` crate + relay-pattern examples | PoC stage 3 must compose from `broadcast`/`rtp-forwarder`; ion-sfu (inactive since 2023) remains an architecture reference |
| Star counts 354/276/98/36/42 | 355 / 363 / 118 / 37 / **123** | talktome undercounted 3x — it is the strongest WebRTC-intercom peer |

## 3. Per-project lessons for TalkServo

1. **talkkonnect — protocol bridges beat purity.** Mumble-to-multicast-RTP-to-SIP bridging delivers hardware interop WebRTC-only stacks can't. TalkServo's D-gateway boundary (SIP/RTP) should follow this seam, and Opus-in-Mumble confirms codec-choice alignment.
2. **talktome — mediasoup is the proven intercom SFU.** Live proof a Node control plane + native SFU worker runs a production talk-lock workflow (tally lights, NDI, program-audio feeds). Validates MediaServo's engine choice as fallback for us; validates our "SFU routes, control plane arbitrates" split.
3. **Gryt — Pion-shaped Go SFU + monorepo discoverability.** Configurable global PTT keybinds and RNNoise show a lightweight 3A posture that works at small scale; our FFI 3A bet is the heavier, more-correct path for mobile AEC.
4. **EVO-PTT — monetization reality check.** Open repo + closed voice server is the common PTT business shape; if TalkServo stays OSS-core, define the commercial boundary early.
5. **Radio-Link — three-tier channel UX** (Duo/Group/Public) is clean; P2P mesh without arbitration confirms why an SFU + real floor model is needed past 2-3 peers.
6. **Zello_Walk — Socket.io audio relay is the anti-pattern** our media layer avoids: no SRTP, no FEC, no jitter control, server becomes the bottleneck.
7. **OpenPTT — stale Cordova; scope of "PTT" in repo names is unreliable.** Always verify before citing.

## 4. Feature benchmark vs. TalkServo targets

| Capability | talkkonnect | EVO-PTT | talktome | Gryt | Radio-Link | **TalkServo target** |
|------------|:-:|:-:|:-:|:-:|:-:|:-:|
| Server-side floor arbitration (deny/queue) | via Mumble | proprietary | talk-lock | partial | implicit | **explicit, modeled, tested** |
| Priority + pre-emption (barge-in) | no | vendor-unknown | no | no | no | **yes — differentiator** |
| Hybrid half/full-duplex switching | channel-based | no | conferences | channels | modes (no arbitration) | **runtime FloorMode switch** |
| WebRTC browser client | no | no | **yes** | **yes** | yes (P2P) | yes |
| SFU media path | mixer-server | proprietary | mediasoup | Pion-Go | none (P2P) | webrtc-rs |
| Multi-language SDK (Rust core) | no | no | no | no | no | **yes — differentiator** |
| Open arbitration core | n/a (Mumble protocol is open, server closed-ish) | closed | open | open | open | Apache/MIT core crate |

No surveyed project combines (a) an explicit floor state machine with (b) priority pre-emption and (c) full-duplex mode in one open core. That intersection is TalkServo's stated niche; the PoC must prove it, not assume it.

## 5. Sources

- GitHub REST API repo metadata, READMEs, and file trees fetched 2026-09-28 (api.github.com; raw.githubusercontent unreachable from build network — README content via API base64).
- Mumble protocol: mumble.info specs (for talkkonnect context).
- pion/webrtc example set: repo tree at master @ snapshot.
- webrtc-rs/webrtc example set: repo tree at master @ snapshot.

## 6. New candidates from the PTT keyword saturation sweep (2026-09-28)

Full table in [github-sweep-ptt.md](github-sweep-ptt.md). Top 5 voice-PTT peers beyond the seven above — deep profiles pending, UNVERIFIED until then:

1. `david-spies/ptt-radio` — browser hold-to-talk, pure P2P WebRTC, "zero audio servers" — the deliberate anti-D2; contrast case for SFU justification
2. `harrowiersma/PTT` — self-hosted TRX-style server with fleet dispatch, GPS, SOS — server feature checklist
3. `devapro` suite (LANwalkieTalkie + ptt-client-android + ptt-server, Kotlin) — closest self-hosted client/server pair to our topology
4. `Eyevinn/intercom-frontend` — low-latency web intercom (TS) — web half-duplex client peer
5. `matiaspl/intercom` — documented Mumble + RPi headless deployment — operational PTT-UX lessons

Naming evidence: `floor control` (1,028 results) and `talk burst` (12) queries returned **zero** relevant voice-arbitration projects — TalkServo's "Floor" vocabulary has no GitHub name collision.
