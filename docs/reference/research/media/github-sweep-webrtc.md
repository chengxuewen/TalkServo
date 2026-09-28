# GitHub Saturation Sweep: WebRTC Media-Server / SFU / Engine Family

> Snapshot date: 2026-09-28. Companion to `media-stack-alternatives.md` and `oss-voice-infrastructure.md`.
> Closes the exhaustive-GitHub gap for the WebRTC media-engine family. All data from the GitHub REST API
> (`api.github.com`, unauthenticated, `sort=stars&per_page=30`); descriptions are verbatim API fields
> (non-ASCII characters redacted to keep this document English-only per C1; truncation at 110 chars).

## 1. Method

- 14 repository-search queries: `"sfu webrtc"`, `"selective forwarding unit"`, `webrtc language:rust`, `webrtc language:zig`, `str0m`, `aiortc`, `werift`, `node-webrtc`, `webrtc-sys`, `"gstreamer webrtcbin"`, `libdatachannel`, `topic:webrtc-sfu`, `mediamtx`, `janus-gateway`.
- Direct repo reads (`GET /repos/{owner}/{repo}`) for existence/status verification, plus base64 README reads for webrtc-rs/sfu, ionorg/ion-sfu, aiortc, shinyorita/werift (404), algestrand/str0m (404).
- 334 unique repositories surfaced; inclusion rule for the master table: stars >= 100 OR last push >= 2024-01-01, plus curated notable small projects; 8 pure-noise rows dropped (listed in section 4).
- Categories: SFU (forwarding server/library), engine-bindings (WebRTC engine implementation or native/FFI binding), gateway (multiprotocol/broadcast/bridge server), muxer (media muxing/codec containers), client-lib (client SDK/app/library), other (apps, tooling, toys, UIs). Rows without an explicit judgment get a keyword-inferred category (from the API description only) - treat as approximate.

## 2. Verification of prior claims (from our notes)

| Claim | Verdict | Evidence (API, 2026-09-28) |
|---|---|---|
| str0m owner = `algestrand/str0m`? | CORRECTED | Actual owner: **`algesten/str0m`** - 627 stars, Rust, "A Sans I/O WebRTC implementation in Rust", pushed 2026-09-27 (active). `algestrand/str0m` and `myscabrd/str0m` both HTTP 404. |
| aiortc SFU demo location | UNVERIFIED | `aiortc/aiortc` exists and is active (5,106 stars, pushed 2026-07-17), but its top-level README (3.5 KB, base64-read via API) contains no "SFU" mention. The often-cited SFU example sits somewhere in the repo examples tree; exact path not confirmed within this sweep's API budget. |
| werift = `shinyorita/werift`? | CORRECTED | `shinyorita/werift` and `werift/werift` both HTTP 404. Actual: **`shinyoshiaki/werift-webrtc`** - 630 stars, TypeScript, ICE/DTLS/SCTP/RTP/SRTP + WebM/MP4 muxing, pushed 2026-09-27 (active). |
| node-webrtc exists/current | CONFIRMED | `node-webrtc/node-webrtc` - 2,804 stars, C++, bindings to WebRTC **M106** (pinned; maintenance-mode signal), pushed 2026-03-26. |
| webrtc-sys = `livekit/webrtc-sys`? | CORRECTED | `livekit/webrtc-sys` HTTP 404 - the crate is a directory inside the **`livekit/rust-sdks`** monorepo (490 stars, pushed 2026-09-27). Third-party forks still reference "livekit/webrtc-sys" as the historical path (hatomist, tartavull). |
| libdatachannel owner | CONFIRMED | `paullouisageneau/libdatachannel` - 2,751 stars, C++, pushed 2026-09-26 (very active). |
| mumble server in `mumble-voip/mumble` | CONFIRMED, with nuance | Repo exists (8,303 stars, pushed 2026-09-26). The Murmur server is **not a separate repository** - client and server share this monorepo. |
| mediasoup | CORRECTED | Canonical org is **`versatica/mediasoup`** (7,377 stars, pushed 2026-09-25). `livekit/mediasoup` HTTP 404. |
| webrtc-rs/sfu | CONFIRMED, SMALL | 82 stars (below the 100 cutoff; missed by every top-30 search, caught only by direct read), pushed 2026-09-19. Active Go reference SFU built on webrtc-rs (sfu.rs project). |
| Gryt | NOT SURFACED | No query returned a Gryt repository; existence UNVERIFIED (owner path unknown; guessing repos is out of scope). |
| thepoison606/talktome | OUT OF SCOPE HERE | PTT keyword family is covered by the sibling sweep (`github-sweep-ptt.md`); none of these 14 queries targeted it. |
| ionorg/ion-sfu status | CONFIRMED, STALE | Exists; 1,100 stars; README: "Go implementation of a WebRTC Selective Forwarding Unit" / "Pure Go WebRTC SFU"; last push 2023-07-21 (stale, not archived). Umbrella `ionorg/ion` (3,800 stars) IS archived (pushed 2023-10-01). |

## 3. Audio-only relay candidates for the TalkServo PoC (ranked)

Aligned with adopted decisions D1 (floor abstraction), D2 (signaling + SFU split), D3 (webrtc-rs engine, no libwebrtc), D4 (webrtc-audio-processing FFI), D5 (4-crate split):

1. **webrtc-rs/webrtc (5,154 stars) + webrtc-rs/sfu (82 stars)** - matches D3 exactly; the sfu reference demonstrates audio/RTP fan-out in the same ecosystem. Caveat: the reference is tiny; expect to harden it into our own relay crate.
2. **algesten/str0m (627 stars)** - sans-I/O design is ideal for driving a D1 floor state machine from our own timer/IO, unit-testable without sockets; pushed on snapshot day. Caveat: RTP payload handling is left to the user. Ecosystem traction visible in str0m-based projects (oxpulse-sfu-kit, lumyx, str0m-e2ee, str0m-stress).
3. **MixinNetwork/kraken (358 stars, Go)** - production audio-only (Opus) SFU; closest published architecture reference for exactly our relay shape, despite language mismatch with D3.
4. **tonarino/webrtc-audio-processing (330 stars, Rust)** - D4 enabler (AEC/NS/AGC bindings), not a relay; pair with rank 1 or 2. Alternatives: Saugat913/webrtc-apm-sys (0-star fresh APM FFI wrapper), nikylogic/webrtc-audio-processing-windows-prebuilt (Windows prebuilt bins for webrtc-audio-processing-sys).
5. **ionorg/ion-sfu** - stale since 2023; architecture reading only.
6. **mediasoup / janus / livekit / mediamtx** - heavyweight general servers; excluded by D2/D3 scope, retained as protocol-behavior oracles (SDP/BWE/ICE semantics references).

Honorable mentions (right shape, pre-alpha): Hisao-Mizuochi-Develop/audio-sfu-server (Rust "ultra-low latency audio SFU", 0 stars, pushed 2026-06-22), anatolykoptev/oxpulse-sfu-kit (str0m-based multi-client SFU kit with simulcast/fanout, 3 stars), mosamorosev/str0m-e2ee (E2EE str0m conferencing, C++), krish9219/openmeet (readable mediasoup SFU, 0 stars).

## 4. Coverage claims

Queries that returned **nothing new** beyond the already-known family (saturation evidence):

- `str0m` (23 hits): only benches/forks/interop toys around `algesten/str0m` (all < 10 stars); no unknown competing engine.
- `webrtc-sys` (56 hits): dominated by tokenization false positives ("webrtc system", "webrtc-System73"); only real signal: `livekit/rust-sdks` placement, two forks (hatomist, tartavull), reactor-team/reactor-webrtc, and the APM prebuilt (nikylogic).
- `werift` (14 hits): canonical repo + interop fixtures only.
- `topic:webrtc-sfu` (13 hits): full overlap - every repo also surfaced in another query.
- `gstreamer webrtcbin` (15 hits): no >= 100-star project; thin wrappers/examples around GStreamer built-ins.
- `janus-gateway` (394 total): beyond canonical Janus, only docker images, client SDKs and plugins of the known server.
- `mediamtx` (594 total): canonical + UI/dashboard/deployment ecosystem; one new notable: winkmichael/mediamtx-moq (Media-over-QUIC fork, 19 stars).
- `aiortc` (220 total): consumer apps/demos built on aiortc; no new server-grade project.
- `selective forwarding unit` (49 total): student/toy SFUs; nothing production-grade new.
- `webrtc language:zig` (8 hits): three nascent Zig WebRTC implementations (zigouat/webrtc 24 stars, alejandroechev/zig-webrtc, bedrock-zenith/nethernet) - all effectively pre-alpha.

**Known blind spots (explicit):** top-30-by-stars truncation hides the tail of `sfu webrtc` (560 total), `webrtc language:rust` (716), `node-webrtc` (2,369), `mediamtx` (594), `janus-gateway` (394). Even `pion/webrtc` was missed by all searches (its description lacks "sfu"/"webrtc-sys" token combos) and surfaced only via direct read - coverage below the ~100-star cutoff is keyword-limited, not saturated.

Dropped as pure noise (met the push-date cutoff, zero signal): Fitruka/Str0mxx, omkadam/one-to-many-webRTC, 17852833820/AioRTC, waterbustech/str0m (fork), Str0mG/Str0mG, str0mback/str0mback, dalton02/Selective-Forwarding-Unit (fork), APhillimore/SSFU.

## 5. Master table (230 repos)

Inclusion rule: stars >= 100 OR pushed >= 2024-01, plus curated notable-small / direct-check rows. Sorted by stars descending. "Query" lists which search(es) surfaced the repo. [ARCHIVED] marks repos with GitHub's archived flag.

| Repo | Stars | Last push | Lang | Description (API verbatim) | Category | Already known | Query |
|---|---|---|---|---|---|---|---|
| jitsi/jitsi-meet | 30,005 | 2026-09-26 | TypeScript | Jitsi Meet - Secure, Simple and Scalable Video Conferences that you use as a standalone app or embed in your w | client-lib | yes (jitsi/meet) | "sfu webrtc" |
| livekit/livekit | 21,154 | 2026-09-28 | Go | End-to-end realtime stack for connecting humans and AI | SFU | yes (livekit) | "sfu webrtc" |
| bluenviron/mediamtx | 20,284 | 2026-09-27 | Go | Ready-to-use Media-over-QUIC / SRT / WebRTC / RTSP / RTMP / LL-HLS / MPEG-TS / RTP live media server and media | gateway | no | mediamtx |
| pion/webrtc | 16,803 | 2026-09-26 | Go | Pure Go implementation of the WebRTC API | engine-bindings | yes (pion/webrtc) | direct repo check |
| meetecho/janus-gateway | 9,170 | 2026-09-23 | C | Janus WebRTC Server | gateway | yes (janus) | janus-gateway |
| mumble-voip/mumble | 8,303 | 2026-09-26 | C++ | Mumble is an open-source, low-latency, high quality voice chat software. | other | yes (mumble) | direct repo check |
| versatica/mediasoup | 7,377 | 2026-09-25 | C++ | Cutting Edge WebRTC Video Conferencing | SFU | yes (mediasoup) | "sfu webrtc" |
| webrtc-rs/webrtc | 5,154 | 2026-09-27 | Rust | Async-friendly WebRTC implementation in Rust | engine-bindings | yes (webrtc-rs) | webrtc language:rust |
| aiortc/aiortc | 5,106 | 2026-07-17 | Python | WebRTC and ORTC implementation for Python using asyncio | engine-bindings | yes (aiortc) | aiortc |
| kunkundi/crossdesk | 4,315 | 2026-09-28 | C++ | A lightweight, cross-platform remote desktop software with support for Web Client access / ???? Web ?????????? [non-English chars redacted] | other | no | libdatachannel |
| ionorg/ion | 3,800 | 2023-10-01 | Go | Real-Distributed  RTC System by pure Go and Flutter | SFU | no | "sfu webrtc" |
| GRVYDEV/Project-Lightspeed | 3,662 | 2023-04-03 | Rust | A self contained OBS -> FTL -> WebRTC live streaming server. Comprised of 3 parts once configured anyone can a | gateway | no | webrtc language:rust |
| miroslavpejic85/mirotalksfu | 3,111 | 2026-09-27 | JavaScript | ? Self-hosted, open-source WebRTC video conferencing platform for real-time communication and collaboration. A [non-English chars redacted] | SFU | no | "sfu webrtc", topic:webrtc-sfu |
| jitsi/jitsi-videobridge | 3,106 | 2026-09-24 | Kotlin | Jitsi Videobridge is a WebRTC compatible video router or SFU that lets build highly scalable video conferencin | SFU | yes (jitsi videobridge) | "sfu webrtc" |
| node-webrtc/node-webrtc | 2,804 | 2026-03-26 | C++ | node-webrtc is a Node.js Native Addon that provides bindings to WebRTC M106 | engine-bindings | yes (node-webrtc) | node-webrtc |
| paullouisageneau/libdatachannel | 2,751 | 2026-09-26 | C++ | C/C++ WebRTC network library featuring Data Channels, Media Transport, and WebSockets | engine-bindings | yes (libdatachannel) | libdatachannel |
| synctv-org/synctv | 2,502 | 2026-09-23 | Rust | Synchronized viewing, theater, live streaming, video | other | no | webrtc language:rust |
| harlanc/xiu | 2,330 | 2026-03-07 | Rust | A simple,high performance and secure live media server in pure Rust (RTMP[cluster]/RTSP/WebRTC[whip/whep]/HTTP [non-English chars redacted] | gateway | no | webrtc language:rust |
| OpenVidu/openvidu | 2,132 | 2026-09-25 | TypeScript | OpenVidu Platform: self-hosted real-time video and audio for your apps, built on LiveKit and mediasoup | gateway | no | "sfu webrtc" |
| peer-calls/peer-calls | 1,900 | 2025-10-28 | Go | Group peer to peer video calls for everyone written in Go and TypeScript | SFU | no | "sfu webrtc" |
| holtwick/briefing | 1,627 | 2025-12-03 | TypeScript | ? Secure direct video group chat [non-English chars redacted] | SFU | no | "sfu webrtc" |
| geckosio/geckos.io | 1,490 | 2026-03-27 | TypeScript | ? Real-time client/server communication over UDP using WebRTC and Node.js [non-English chars redacted] | client-lib | no | node-webrtc |
| jech/galene | 1,410 | 2026-09-25 | Go | The Gal?ne videoconference server [non-English chars redacted] | SFU | no | "sfu webrtc" |
| edumeet/edumeet | 1,349 | 2026-09-25 | Shell | edumeet - multiparty web-meetings using mediasoup and WebRTC | SFU | no | "sfu webrtc" |
| open-webrtc-toolkit/owt-server | 1,155 | 2024-10-23 | JavaScript | General server (streaming/conference/transcoding/anayltics) for OWT. (A.k.a. MediaServer) | SFU | no | "sfu webrtc" |
| johanhelsing/matchbox | 1,152 | 2026-06-02 | Rust | Painless peer-to-peer WebRTC networking for rust wasm (and native!) | client-lib | no | webrtc language:rust |
| ionorg/ion-sfu | 1,100 | 2023-07-21 | Go | Pure Go WebRTC SFU | SFU | yes (ionorg/ion-sfu) | "sfu webrtc" |
| mat-sz/filedrop | 964 | 2026-04-25 | TypeScript | ? WebRTC E2E encrypted file transfer - React + node.js [non-English chars redacted] | client-lib | no | node-webrtc |
| restsend/rustpbx | 812 | 2026-09-28 | Rust | A PBX written by rust | gateway | no | webrtc language:rust |
| bitwhip/bitwhip | 728 | 2024-11-04 | Rust | CLI Native WebRTC Agent in Rust | client-lib | no | webrtc language:rust |
| amirsanni/Video-Call-App-NodeJS | 703 | 2026-09-03 | JavaScript | A conference call implementation using WebRTC, Socket.io and Node.js | client-lib | no | node-webrtc |
| atyenoria/janus-webrtc-gateway-docker | 692 | 2023-01-09 | Dockerfile | Perfect Docker Image for Media Streaming Expert User ( https://github.com/meetecho/janus-gateway ) | other | no | janus-gateway |
| gethopp/hopp | 691 | 2026-09-20 | Rust | The best OSS remote pair programming app. | other | no | webrtc language:rust |
| versatica/mediasoup-client | 669 | 2026-09-24 | TypeScript | mediasoup client side JavaScript library | client-lib | yes (mediasoup client) | "sfu webrtc" |
| shinyoshiaki/werift-webrtc | 630 | 2026-09-27 | TypeScript | WebRTC Implementation for TypeScript (Node.js), includes ICE/DTLS/SCTP/RTP/SRTP/WEBM/MP4 | engine-bindings | yes (werift; owner corrected) | werift, node-webrtc |
| algesten/str0m | 627 | 2026-09-27 | Rust | A Sans I/O WebRTC implementation in Rust. | engine-bindings | yes (str0m; owner corrected) | webrtc language:rust, str0m |
| notedit/media-server-go | 554 | 2020-07-02 | Go | WebRTC media server for go | SFU | no | "sfu webrtc" |
| node-webrtc/node-webrtc-examples | 544 | 2022-09-09 | JavaScript | MediaStream and RTCDataChannel examples using node-webrtc | other | no | node-webrtc |
| webtorrent/webtorrent-hybrid | 529 | 2026-05-25 | JavaScript | WebTorrent (with WebRTC support in Node.js) | other | no | node-webrtc |
| ccallcn/ovsyunlive | 524 | 2026-09-20 | JavaScript | ????rtsp,Web??rtmp??,Web??rtsp/rtmp?????,SIP??,????,??????,????,????,MCU/SFU????,????,rtsp??,????,?eb??,flv??? [non-English chars redacted] | gateway | no | "sfu webrtc" |
| mycrl/turn-rs | 513 | 2026-09-14 | Rust | A pure rust implemented turn server. | other | no | webrtc language:rust |
| livekit/rust-sdks | 490 | 2026-09-27 | Rust | LiveKit realtime and server SDKs for Rust | engine-bindings | yes (webrtc-sys lives here) | webrtc language:rust |
| meething/meething | 461 | 2024-06-18 | JavaScript | dWebRTC Video Meetings MESH/SFU hybrid using GunDB, MediaSoup and Beyond! | SFU | no | "sfu webrtc" |
| jangouts/jangouts | 446 | 2023-11-13 | JavaScript | Videoconferencing based on WebRTC and Janus Gateway with an UI inspired by Google Hangouts | client-lib | no | janus-gateway |
| murat-dogan/node-datachannel | 419 | 2026-09-12 | C++ | WebRTC For Node.js and Electron (including WebSocket Client & Server). libdatachannel node bindings. | engine-bindings | no | libdatachannel, node-webrtc |
| kyren/webrtc-unreliable | 418 | 2025-03-15 | Rust | Just enough hacks to get unreliable unordered WebRTC data channels between a browser and a server | engine-bindings | no | webrtc language:rust |
| kstonekuan/tambourine-voice | 386 | 2026-07-17 | Rust | Your personal voice interface for any app. Speak naturally and your words appear wherever your cursor is, with | other | no | webrtc language:rust |
| ddssingsong/webrtc_server_node | 384 | 2026-03-30 | JavaScript | videoCall  VideoConference ???? ???? [non-English chars redacted] | client-lib | no | node-webrtc |
| MixinNetwork/kraken | 358 | 2026-08-04 | Go | ? High performance WebRTC audio SFU implemented with pure Go. [non-English chars redacted] | SFU | no | "sfu webrtc" |
| waterbustech/waterbus | 340 | 2025-09-18 | Dart | Showcase app demonstrating Waterbus-powered real-time media. | client-lib | no | "sfu webrtc" |
| 8xFF/atm0s-media-server | 330 | 2026-09-25 | Rust | Decentralized, Global-Scale Media Server written in Rust (WebRTC/Whip/Whep/Rtmp/Sip) | gateway | no | "sfu webrtc", webrtc language:rust |
| tonarino/webrtc-audio-processing | 330 | 2026-07-16 | Rust | Rust bindings for the webrtc-audio-processing library | engine-bindings | no | webrtc language:rust |
| mappum/electron-webrtc | 316 | 2018-02-20 | JavaScript | ? Use WebRTC in Node.js via a hidden Electron process [non-English chars redacted] | engine-bindings | no | node-webrtc |
| binbat/live777 | 311 | 2026-09-28 | Rust | Live777 Media Server. A very simple, high performance, edge WebRTC SFU | SFU | no | "sfu webrtc", topic:webrtc-sfu, webrtc language:rust |
| gjovanov/roomler | 294 | 2026-05-01 | JavaScript | Roomler - Multi-party Video Conferencing & Team Collaboration Tool using WebRTC (Janus Gateway) | client-lib | no | janus-gateway |
| adrigardi90/video-chat | 283 | 2020-06-15 | JavaScript | Video chat app using Vue, Vuex, WebRTC, SocketIO, Node, Redis & Docker with horizontal scaling. Multiparty and | client-lib | no | node-webrtc |
| Dirvann/mediasoup-sfu-webrtc-video-rooms | 273 | 2024-04-18 | JavaScript | A simple video conferencing example using the mediasoup sfu | client-lib | no | "sfu webrtc" |
| takahirox/nes-rust | 230 | 2020-08-28 | Rust | NES emulator written in Rust + WASM | other | no | webrtc language:rust |
| medooze/sfu | 224 | 2022-02-22 | JavaScript | A future proof, experimental WebRTC VP9 SVC SFU wit end to end encryption support | SFU | no | "sfu webrtc" |
| RingsNetwork/rings | 220 | 2026-09-27 | Rust | Rings is a structured peer-to-peer network implementation using WebRTC, Chord DHT, and full WebAssembly (WASM) | other | no | webrtc language:rust |
| restsend/rsipstack | 210 | 2026-09-28 | Rust | SIP Stack Rust library for building SIP applications | client-lib | no | webrtc language:rust |
| IceDBorn/pipewire-screenaudio | 200 | 2026-08-09 | Rust | Extension to passthrough pipewire audio to WebRTC Screenshare | engine-bindings | no | webrtc language:rust |
| atyenoria/react-native-webrtc-janus-gateway | 198 | 2018-03-30 | JavaScript | Video conference system for mobile application.  Base technology is react-native-webrtc + Janus Webrtc Gateway | client-lib | no | janus-gateway |
| paullouisageneau/datachannel-wasm | 198 | 2026-09-27 | C++ | C++ WebRTC Data Channels and WebSockets for WebAssembly in browsers | engine-bindings | no | libdatachannel |
| miuda-ai/active-call | 190 | 2026-09-23 | Rust | A SIP/WebRTC voice agent | gateway | no | webrtc language:rust |
| ouxianghui/janus-client | 189 | 2022-04-06 | C++ | c/c++ webrtc native janus client Qt opengl video-meeting video-room video-call text-room meeting chat | client-lib | no | janus-gateway |
| meeting-rs/meeting.rs | 186 | 2026-05-29 | Rust | Private one to one realtime video meeting.? [non-English chars redacted] | client-lib | no | webrtc language:rust |
| canyanio/janus-gateway-docker | 185 | 2026-05-12 | Dockerfile | Docker image for the Janus WebRTC Server | other | no | janus-gateway |
| sjkummer/janus-gateway-js | 180 | 2024-09-17 | JavaScript | Janus-gateway WebRTC client for Node.js and the browser. | client-lib | no | janus-gateway, node-webrtc |
| yanhua133/mediasoup-sfu-cpp | 178 | 2023-04-07 | C++ | webrtc c++ library for mediasoup with full sfu c++ demo | SFU | no | "sfu webrtc" |
| mozilla/webrtc-sdp | 165 | 2026-09-25 | Rust | Rust SDP parser for WebRTC | engine-bindings | no | webrtc language:rust |
| webrtc-rs/rtc | 164 | 2026-09-27 | Rust | Sans-IO WebRTC implementation in Rust | engine-bindings | no | webrtc language:rust |
| Charles-Schleich/WebRTC-in-Rust | 161 | 2023-05-12 | Rust | A source code for a working WebRTC project, written in head to toe Rust. | other | no | webrtc language:rust |
| jettbrains/-L- | 160 | 2021-08-18 | - | W3C Strategic Highlights  September 2019  This report was prepared for the September 2019 W3C Advisory Committ [non-English chars redacted] | other | no | node-webrtc |
| kunkundi/minirtc | 153 | 2026-09-28 | C++ | A lightweight cross-platform real-time audio and video transmission engine / ????????????????? [non-English chars redacted] | engine-bindings | no | libdatachannel |
| januscaler/flutter_janus_client | 152 | 2026-08-20 | Dart | A plugin that allows the flutter app to communicate with a Janus server using different transport mechanisms,  | client-lib | no | janus-gateway |
| webRTCv1/Best-of-webRTC | 152 | 2026-09-17 | - | Best of WebRTC: Elevate with Top Projects! ?? ?? ?? ?? ?? [non-English chars redacted] | other | no | "sfu webrtc" |
| lerouxrgd/datachannel-rs | 151 | 2026-07-27 | Rust | Rust wrappers for libdatachannel | engine-bindings | no | libdatachannel, webrtc language:rust |
| Eyevinn/srt-whep | 142 | 2026-09-24 | Rust | SRT to WHEP (WebRTC) | gateway | no | webrtc language:rust |
| membraneframework-labs/membrane_rtc_engine | 139 | 2024-09-19 | Elixir | Customizable Real-time Communication Engine/SFU library focused on WebRTC. | SFU | no | "sfu webrtc" |
| mozilla/janus-plugin-sfu | 136 | 2026-02-03 | Rust | Janus plugin to act as a kind of SFU for game networking data. | SFU | no | janus-gateway |
| feixiao/learning_webrtc | 133 | 2020-07-15 | JavaScript | webrtc????(???????) [non-English chars redacted] | other | no | janus-gateway |
| wasm-peers/wasm-peers | 132 | 2024-02-04 | Rust | Easy-to-use wrapper for WebRTC DataChannels peer-to-peer connections written in Rust and compiling to WASM. | engine-bindings | no | webrtc language:rust |
| codec-abc/Yew-WebRTC-Chat | 128 | 2023-08-17 | Rust | A simple WebRTC chat made with Yew | client-lib | no | webrtc language:rust |
| meetecho/simple-whip-server | 124 | 2026-02-10 | JavaScript | Node.js Simple WHIP Server library (based on the Janus WebRTC Server) | gateway | no | node-webrtc |
| revoltchat/vortex | 121 | 2024-03-28 | Rust | (in development) Pluggable WebRTC Voice Server | SFU | no | webrtc language:rust |
| benwtrent/janus-gateway-android | 113 | 2016-04-01 | Java | This is an API wrapper that utilizes the native WebRTC build and is made to ease communication with the janus- | client-lib | no | janus-gateway |
| meetecho/janode | 113 | 2026-09-25 | JavaScript | A Node.js adapter for the Janus WebRTC server | client-lib | no | node-webrtc |
| RobbieXie/WebRTC-Classroom | 110 | 2023-10-27 | JavaScript | ??WebRTC???????? ??NODE?RTMP??  This is a webrtc demo for teachers and students, teaching by live camera & scr [non-English chars redacted] | client-lib | no | node-webrtc |
| restsend/rustrtc | 110 | 2026-09-23 | Rust | A high-performance implementation of WebRTC. | engine-bindings | no | "sfu webrtc" |
| Hyunse/video-group-meeting | 106 | 2021-12-04 | JavaScript | WebRTC video chat for multi users using React and Node Express. | client-lib | no | node-webrtc |
| bluenviron/mediacommon | 100 | 2026-09-27 | Go | Entities shared between gohlslib, gortsplib, gortmplib, MediaMTX | muxer | no | mediamtx |
| borjanebbal/webrtc-node-app | 100 | 2026-09-03 | JavaScript | This repository contains a simple WebRTC app, created for educational purposes. | client-lib | no | node-webrtc |
| inlivedev/sfu | 99 | 2026-03-07 | Go | WebRTC Selective Forwarder Unit(SFU) Golang Library | SFU | no | "sfu webrtc" |
| runner365/RTCPilot | 98 | 2026-06-04 | C++ | RTCPilot is an open-source WebRTC SFU (Selective Forwarding Unit) implemented in modern C++. support win11/lin | SFU | no | "sfu webrtc", "selective forwarding unit" |
| ambianic/peerjs-python | 95 | 2024-01-31 | Python | Python port of PeerJS client | client-lib | no | aiortc |
| michaelfranzl/janus-rtpforward-plugin | 87 | 2024-01-06 | C | Plugin for Janus forwarding RTP and RTCP packets to an external UDP receiver/decoder, e.g. a GStreamer pipelin | gateway | no | janus-gateway |
| meetecho/simple-whip-client | 84 | 2025-10-23 | C | Simple WHIP Client (based on GStreamer's webrtcbin) | client-lib | no | "gstreamer webrtcbin" |
| webrtc-rs/sfu | 82 | 2026-09-19 | Rust | Sans-IO SFU implementation in Rust | SFU | yes (webrtc-rs/sfu) | direct repo check |
| bcanfield/mediamtx-connect | 80 | 2026-09-27 | TypeScript | Web UI for MediaMTX. Watch streams, browse recordings, and edit config from your browser. | other | no | mediamtx |
| takwerx/infra-TAK | 80 | 2026-09-27 | Python | Browser based Team Awareness Kit Infrastructure Management Platform. One-click deployment of TAK Server, Authe | other | no | mediamtx |
| jameskitt616/vrchat_streaming | 79 | 2026-05-01 | - | Tool to stream anything (e.g. Emby/Plex/Jellyfin etc.) to VRChat | other | no | mediamtx |
| aljanabim/simple_webrtc_signaling_server | 71 | 2025-04-06 | JavaScript | A WebRTC signaling server implemented in Node.js with Socket.io | client-lib | no | node-webrtc |
| versatica/mediasoup-client-aiortc | 71 | 2026-09-13 | TypeScript | mediasoup-client handler for aiortc Python library | engine-bindings | yes (mediasoup x aiortc) | aiortc |
| seekwhencer/mediamtx-ui | 63 | 2026-02-02 | JavaScript | Depenency free (so far) Vanilla JS Dashboard UI for the mediamtx streaming server. Dockerized. | other | no | mediamtx |
| twtrubiks/django-chat-room | 60 | 2026-07-17 | Python | Django Channels 4 ???????? MediaMTX ???????WebRTC < 1 ????? ? ?????????????? [non-English chars redacted] | other | no | mediamtx |
| wangsrGit119/janus-webrtc-gateway-docker | 60 | 2026-04-28 | Dockerfile | Docker image for the Janus WebRTC Server;Janus docker ?? ?? [non-English chars redacted] | other | no | janus-gateway |
| PsymoNiko/mediamtx-dashboard | 56 | 2026-08-31 | TypeScript | ? Modern web dashboard for MediaMTX ? real-time stream monitoring, config management, and built-in Prometheus/ [non-English chars redacted] | other | no | mediamtx |
| supersjgk/LiveStream-WebRTC-Flask-OpenCV | 43 | 2024-09-09 | JavaScript | A simple Live Streaming Flask app that uses WebRTC (aiortc) and OpenCV | client-lib | no | aiortc |
| karlcswanson/feedboard | 42 | 2026-01-10 | TypeScript | Web based toolkit for MediaMTX | other | no | mediamtx |
| lgcshy/mediamtx-ui | 30 | 2026-04-19 | Vue | Modern admin dashboard for MediaMTX streaming server ? Vue 3 + TypeScript + Element Plus + ECharts [non-English chars redacted] | other | no | mediamtx |
| medooze/libdatachannels | 30 | 2024-10-01 | C++ | Lean and mean WebRTC datachannels C++ library with ad-hoc SCTP stack | engine-bindings | no | libdatachannel |
| odoo/sfu | 28 | 2026-09-23 | TypeScript | Odoo's Selective Forwarding Unit | SFU | no | "selective forwarding unit" |
| parallelcc/FFmpeg-WHIP-WHEP | 28 | 2025-05-19 | C | Enable WHIP/WHEP support in FFmpeg using libdatachannel | engine-bindings | no | libdatachannel |
| bluenviron/mediamtx-rpicamera | 27 | 2026-09-05 | C | Raspberry Pi Camera component for MediaMTX | other | no | mediamtx |
| zigouat/webrtc | 24 | 2026-09-27 | Zig | WebRTC implementation in zig | engine-bindings | no | webrtc language:zig |
| takwerx/mediamtx-installer | 22 | 2026-09-12 | Python | MediaMTX video streaming platform with web-based configuration editor ? RTSP/RTMP/HLS/SRT support for ISR dron [non-English chars redacted] | other | no | mediamtx |
| 855princekumar/PiStream-Lite | 21 | 2026-04-05 | Shell | One-command RTSP streaming setup for Raspberry Pi 3B+, Pi 4, and Pi 5. USB webcam ? H.264 RTSP stream with aut [non-English chars redacted] | other | no | mediamtx |
| HuangRunHua/LatteCam | 21 | 2026-06-25 | Swift | Turn an old iPhone into a secure local HomeKit camera using SwiftUI, RTMPS, MediaMTX, Scrypted, and HomeKit. | other | no | mediamtx |
| damionrashford/media-os | 20 | 2026-05-31 | Python | Routed Media OS for Claude Code ? 96 skills + 13 modes + 7 specialist agents covering FFmpeg, OBS, NDI, DeckLi [non-English chars redacted] | other | no | mediamtx |
| elementtime6969/mycam | 20 | 2026-09-27 | PowerShell | MYCAM Virtual Camera for Android ? standalone rooted-phone app for local media or OBS RTMP streams, Watch mode [non-English chars redacted] | client-lib | no | mediamtx |
| winkmichael/mediamtx-moq | 19 | 2025-10-28 | Go | MediaMTX fork adding MoQ protocol support for ultra-low latency streaming (<300ms). WebTransport + native QUIC | gateway | no | mediamtx |
| MostlyBuilds/op25-radio-stream | 16 | 2026-05-06 | Python | OP25-based P25 radio decoder for RTL-SDR that runs in Docker and streams audio as an always-on RTSP/HLS feed t | other | no | mediamtx |
| pschichtel/libdatachannel-java | 16 | 2026-09-27 | C++ | (none) | other | no | libdatachannel |
| shiguredo/sora-c-sdk | 16 | 2025-05-11 | C++ | WebRTC SFU Sora C SDK | client-lib | no | libdatachannel |
| tomtom215/LyreBirdAudio | 16 | 2026-07-21 | Shell | This is a personal tool to try to optimize the installation of MediaMTX to create RTSP audio streams based on  | other | no | mediamtx |
| wuha-xt/mpp_RTSP_stream_demo | 16 | 2024-10-12 | C++ | RTSP to RTSP stream demo. using ffmpeg-mpp, mediaMTX. pull->decode->encode->push | client-lib | no | mediamtx |
| meetecho/simple-whep-client | 15 | 2026-02-10 | C | Simple WHEP Client (based on GStreamer's webrtcbin) | client-lib | no | "gstreamer webrtcbin" |
| AakashBhat1/argus | 13 | 2026-09-25 | Python | AI-powered intruder detection system (YOLO + ViT crime classifier + streaming + chatbot) | client-lib | no | mediamtx |
| FiLORUX/mtx-mon | 13 | 2026-06-10 | HTML | MediaMTX Monitor GUI | other | no | mediamtx |
| darton/RPiMS | 12 | 2026-04-02 | Python | Raspberry Pi Monitoring System offering video streaming (MediaMTX), support for GPIO/digital sensors, motion a | other | no | mediamtx |
| waterbustech/waterbus-server-ws | 12 | 2024-10-26 | TypeScript | Open source video conferencing app built on latest WebRTC SDK. This is Server SFU WebSocket. | client-lib | no | topic:webrtc-sfu |
| MinChanSike/mediamtx-client | 11 | 2026-09-21 | TypeScript | Portable browser console, live WebRTC/HLS multi-viewer, and synchronized multi-stream playback client for Medi | client-lib | no | mediamtx |
| farseenmanekhan1232/neustream | 10 | 2026-03-09 | TypeScript | Neustream is a open-source alternative to platforms like Restream.io, StreamYard, and Castr. | other | no | mediamtx |
| oviceinc/mediasoup-elixir | 10 | 2026-09-24 | Elixir | Mediasoup port for Elixir | other | no | topic:webrtc-sfu |
| prothegee/zix | 10 | 2026-09-05 | Zig | A high-performance network backend library & http engine written in Zig. Where the wire meets the will. Every  | engine-bindings | no | webrtc language:zig |
| WebRTSP/RtStreaming | 9 | 2026-09-01 | C++ | Helper library intended to simplify use GStreamer's `webrtcbin` in C++ applications. | engine-bindings | no | "gstreamer webrtcbin" |
| mitant/aiortc-picamera2-webrtc | 8 | 2024-03-28 | Python | webrtc implementation that mimics camera-streamer webrtc negotiation. useful for raspberry pi 5 & ratrig vcore | other | no | aiortc |
| q-qo-o/Webrtc_Opencv_Demo | 8 | 2024-04-20 | Python | ???opencv?webrtc?????upd?aiortc?pyav?opencv-python ???????? [non-English chars redacted] | other | no | aiortc |
| bubochka14/ChatUp | 7 | 2026-05-11 | C++ | Desktop messenger with video/audio calls on Qt6 | other | no | libdatachannel |
| midoelhawy/mediamtx-node-client | 7 | 2025-04-24 | TypeScript | (none) | other | no | mediamtx |
| moritztng/cellular | 7 | 2024-08-01 | Python | Cellular Automata in PyTorch with Multiplayer Mode in the Browser via WebRTC :video_game: | other | no | aiortc |
| digital-divas/Onvif-IP-Camera-Mock | 6 | 2026-04-12 | Rust | A lightweight IP camera simulator written in Rust, capable of generating a real-time RTSP stream and designed  | other | no | mediamtx |
| dud1337/TiNiStRiMi | 6 | 2025-02-08 | Shell | Simple streaming platform. OBS/Streamlabs compatible. View in browser. Sub-second latency. | other | no | mediamtx |
| maileshwaran28/media-routing-mesh | 6 | 2026-09-28 | HTML | Routed Media OS 2026: 96 AI Skills, 13 Modes & 7 Agents for FFmpeg, OBS, DeckLink & Dolby Vision | other | no | mediamtx |
| reactor-team/reactor-webrtc | 6 | 2026-09-15 | Rust | Reactor's owned WebRTC stack (safe Rust API + sys + build pipeline). Shared by the client SDKs and the runtime | engine-bindings | no | webrtc-sys |
| supertigim/video_streaming_on_web | 6 | 2026-02-13 | JavaScript | Web Cam Video Streaming using aiohttp, aiortc, OpenCV, WebRTC, React and etc | other | no | aiortc |
| Bedrock-Phanatics/nethernet-zig | 5 | 2026-09-26 | Zig | High-performance, low-latency, and secure NetherNet implementation for Minecraft: Bedrock Edition | other | no | webrtc language:zig |
| anatolykoptev/oxpulse-partner-edge | 5 | 2026-09-18 | Shell | OxPulse partner-edge deployment bundle ? Caddy, coturn, xray, and str0m-based SFU for partners to self-host ox [non-English chars redacted] | SFU | no | str0m |
| matiasdelellis/eneverre-android | 5 | 2026-08-16 | Java | Yet another rtsp/onvif/onvif?/thingino client to manage cameras.. | client-lib | no | mediamtx |
| mmkhitaryan/nektobot | 5 | 2024-07-04 | Python | Bidirectional Nekto.me <-> Discord voice bot using aiortc and discord-py | client-lib | no | aiortc |
| nimbase/libdatachannel-nim | 5 | 2026-09-26 | Nim | Nim bindings for Libdatachannel ?? A standalone WebRTC Data Channels, WebRTC Media Transport, and WebSockets l [non-English chars redacted] | engine-bindings | no | libdatachannel |
| TLabAltoh/Unity-SFU-Integration | 3 | 2025-03-18 | C# | Test project for implementing WebSocket / WebRTC's SFU (Selectable Forwarding Unit) network architecture on th | SFU | no | topic:webrtc-sfu |
| anatolykoptev/oxpulse-sfu-kit | 3 | 2026-07-17 | Rust | Reusable multi-client SFU kit built on top of str0m. Simulcast, fanout, per-peer event routing. | SFU | no | str0m |
| dorisoy/Mediasoup.SFU | 3 | 2025-12-11 | C++ | ????C++17??????SFU?Selective Forwarding Unit???????mediasoup???????????JavaScript????????????Oatpp?????Web???? [non-English chars redacted] | SFU | no | "selective forwarding unit" |
| shiguredo/libdatachannel-py | 3 | 2026-08-30 | C++ | Python bindings for libdatachannel | engine-bindings | no | libdatachannel |
| sjhorn/webrtc_dart | 3 | 2026-01-18 | Dart | A pure dart webrtc implementation based on werift | engine-bindings | no | werift |
| Er-Sadiq/Real-time_Hand_Gesture_Recognition_System-with-OpenCV-WebRTC | 2 | 2024-03-28 | Python | (none) | other | no | webrtc-sys |
| alloverse/AlloDataChannel | 2 | 2026-09-03 | C++ | Swift wrapper around libdatachannel, a WebRTC implementation. | other | no | libdatachannel |
| silbinarywolf/zig-libdatachannel | 2 | 2026-04-25 | C | Zig Bindings for libdatachannel (C/C++ WebRTC Network Library) | engine-bindings | no | libdatachannel |
| webrtc-node/webrtc | 2 | 2026-08-13 | JavaScript | WebRTC data channels for Node.js, backed by libdatachannel and validated with 620 selected Web Platform Tests  | engine-bindings | no | libdatachannel |
| wusspuss/libdatachannel-python | 2 | 2024-03-22 | Python | (none) | other | no | libdatachannel |
| xnorpx/str0m_browser_integration_tests | 2 | 2026-08-06 | Rust | Test str0m interop with browsers | other | no | str0m |
| zllovesuki/esp32-radio | 2 | 2026-09-18 | Rust | A pocket radio built with ESP32-S3 and Rust, streaming music to browsers over WebRTC. | other | no | str0m |
| FrekiManagarm/lumyx | 1 | 2026-09-22 | JavaScript | ? Open-source WebRTC SFU in Rust, with observability built into the media path instead of bolted on. Alpha. [non-English chars redacted] | SFU | no | topic:webrtc-sfu, str0m |
| HaishinKit/libdatachannel-xcframework | 1 | 2026-09-27 | Python | libdatachannel with xcframework project | other | no | libdatachannel |
| NostrGameEngine/libdatachannel-java | 1 | 2026-05-30 | Java | A fork of libdatachannel C Java wrappers: libdatachannel-java intended to be used within the Nostr Game Engine | other | no | libdatachannel |
| Raman990609/P2PDemo | 1 | 2025-07-08 | C++ | a p2p demo with libdatachannel | client-lib | no | libdatachannel |
| bedrock-zenith/nethernet | 1 | 2026-09-17 | Zig | Minimal WebRTC implementation that is required for running nethernet | client-lib | no | webrtc language:zig |
| cephalofi/DataChannelDotnet | 1 | 2025-09-05 | - | C# WebRtc library (Libdatachannel wrapper) | engine-bindings | no | libdatachannel |
| cgeffect/rtc-datachannel | 1 | 2025-08-20 | HTML | libdatachannel learn | other | no | libdatachannel |
| dkprog/webrtc-grid-demo-livekit | 1 | 2026-06-03 | TypeScript | WebRTC fan-out demo supporting N producers and N consumers using LiveKit as an SFU, with a GStreamer example f | SFU | no | topic:webrtc-sfu |
| f2rkan/webrtc-gstreamer-stream | 1 | 2024-08-28 | Vue | WebRTC and GStreamer integration using werift-webrtc for real-time audio and video streaming, enabling low-lat | other | no | werift |
| gx14ac/icez | 1 | 2026-07-29 | Zig | Pure Zig ICE (RFC 8445) implementation | engine-bindings | no | webrtc language:zig |
| maiguangyang/relay_core | 1 | 2026-02-02 | Dart | ?? Pion WebRTC ?????? SFU?Selective Forwarding Unit?????? Dart FFI ??????? RTP ????????????????????????? [non-English chars redacted] | SFU | no | "selective forwarding unit" |
| nikylogic/webrtc-audio-processing-windows-prebuilt | 1 | 2026-06-25 | - | Precompiled Windows binaries (MSVC/x86_64) for webrtc-audio-processing-sys to bypass Meson, Ninja, and applefr | engine-bindings | no | webrtc-sys |
| slightknack/cooperate | 1 | 2025-01-11 | Zig | Work in progress. What if you could use a website with your friends? CRDTs + Zig + Wasm + WebRTC. | client-lib | no | webrtc language:zig |
| yhbsh/sfu | 1 | 2026-04-11 | C | Selective Forwarding Unit | SFU | no | "selective forwarding unit" |
| ADIthaker/XDP_SFU | 0 | 2025-01-04 | C | ??Selective Forwarding Unit Implementation using XDP and TC hooks [non-English chars redacted] | SFU | no | "selective forwarding unit" |
| Abhisek0721/video-calling-mediasoup | 0 | 2025-02-15 | TypeScript | Video Conferencing -One to One (SFU - Selective Forwarding Unit architecture) using mediasoup and socket.io | SFU | no | "selective forwarding unit" |
| Dyastin-0/echos | 0 | 2026-09-12 | Go | Video conferencing using Pion | other | no | topic:webrtc-sfu |
| Ellipse-0806/webrtc-with-wasm | 0 | 2024-01-28 | - | Wrapper for gstreamer's webrtcbin element. | other | no | "gstreamer webrtcbin" |
| EngineerFaunce/str0m-intro | 0 | 2026-03-10 | Rust | A side project to learn about rust and WebRTC. | other | no | str0m |
| Hisao-Mizuochi-Develop/audio-sfu-server | 0 | 2026-06-22 | Rust | Ultra-low latency audio SFU (Selective Forwarding Unit) server | SFU | no | "selective forwarding unit" |
| Igorpcferreira/webRTC_IgorFerreira | 0 | 2026-06-11 | C | Minimal WebRTC audio and video transfer between two processes using C, GStreamer and webrtcbin. | other | no | "gstreamer webrtcbin" |
| Roshan23R/webrtc-surveillance-system | 0 | 2026-06-29 | TypeScript | Real-Time Camera Surveillance Dashboard with Person  Detection (WebRTC) | other | no | webrtc-sys |
| Saugat913/webrtc-apm-sys | 0 | 2025-10-21 | C++ | Webrtc audio processing module ffi wrapper | engine-bindings | no | webrtc-sys |
| Su9DenLY/webrtc-conference-system | 0 | 2026-04-27 | TypeScript | (none) | other | no | webrtc-sys |
| System73/system73-webrtc-ios-spm | 0 | 2026-07-03 | Swift | Swift Package Manager distribution for the System73 WebRTC iOS XCFramework. | other | no | webrtc-sys |
| System73/system73-webrtc-tvos-spm | 0 | 2026-07-03 | Swift | Swift Package Manager distribution for the System73 WebRTC tvOS XCFramework. | other | no | webrtc-sys |
| TamasSolent/webrtc-audience-system | 0 | 2026-03-20 | - | AudienceQ ? Real-time WebRTC Live Q&A Platform [non-English chars redacted] | other | no | webrtc-sys |
| Teyk0o/str0m-stress | 0 | 2026-04-04 | Rust | Open-source stress-test tool for WebRTC SFU, based on str0m. | other | no | str0m |
| ThanhDodeurOdoo/str0m-benchmarks | 0 | 2026-07-03 | Rust | (none) | other | no | str0m |
| Toru-de/webrtc-system | 0 | 2025-03-29 | HTML | (none) | other | no | webrtc-sys |
| UE2020/libdatachannel-str0m-compat | 0 | 2024-08-05 | C++ | Minimum reproducible example | other | no | str0m |
| Vall98/SFU | 0 | 2024-11-22 | C++ | A Selective Forwarding Unit media server implemented in C++ | SFU | no | "selective forwarding unit" |
| akshay-ios/system73-webrtc-spm | 0 | 2026-06-25 | Swift | webrtc package | other | no | webrtc-sys |
| akshay-ios/system73-webrtc-tvos-spm | 0 | 2026-06-25 | Swift | (none) | other | no | webrtc-sys |
| alejandroechev/zig-webrtc | 0 | 2026-04-22 | Zig | A WebRTC library for Zig, built using structured RFC rules as specification | engine-bindings | no | webrtc language:zig |
| alsalemalhalimi/webrtc-exam-system | 0 | 2025-11-25 | - | ???? ?????? ?????????? ???????? WebRTC [non-English chars redacted] | other | no | webrtc-sys |
| anshumandev2025/echomeet | 0 | 2026-04-19 | TypeScript | Echomeet is a Google Meet clone built using the MediaSoup SFU (Selective Forwarding Unit) architecture. | SFU | no | "selective forwarding unit" |
| blax-software/laravel-webrtc | 0 | 2026-08-11 | PHP | WebRTC for Laravel on the blax ReactPHP kernel: signaling on the shared loop + a pluggable media engine (Rust/ | other | no | str0m |
| cyberwlodarczyk/zoom | 0 | 2025-10-17 | Rust | WebRTC Selective Forwarding Unit | SFU | no | "selective forwarding unit" |
| davibe/str0m-benches | 0 | 2024-02-08 | Rust | (none) | other | no | str0m |
| dkprog/webrtc-grid-demo | 0 | 2026-05-11 | TypeScript | WebRTC fan-out demo supporting N producers and N consumers, with a GStreamer example for injecting video into  | client-lib | no | "gstreamer webrtcbin" |
| gorrotowi/webmulator | 0 | 2026-08-21 | JavaScript | Stream Android emulator and iOS Simulator screens to the browser over WebRTC (werift), with gRPC / WebDriverAg | other | no | werift |
| harshadkhetpal/webrtc-audience-system | 0 | 2026-03-22 | JavaScript | AudienceQ ? Real-time WebRTC Live Q&A Platform with queue management, live translation, analytics and projecto [non-English chars redacted] | other | no | webrtc-sys |
| hatomist/webrtc-sys | 0 | 2026-02-16 | Assembly | Fork of livekit/webrtc-sys with HKDF-SHA256 key derivation for E2EE interop with JS SDK | engine-bindings | no | webrtc-sys |
| huishanwong/webrtc-interpretation-system | 0 | 2026-09-22 | HTML | Local network interpretation system with Node.js and WebRTC | other | no | webrtc-sys |
| imdotrino/dotrino-webrtc | 0 | 2026-09-03 | JavaScript | WebRTC en JavaScript puro para Dotrino: solo canales de datos. Poda de werift. | other | no | werift |
| imentus-divya/GoSFU | 0 | 2025-07-24 | - | A Go-based Selective Forwarding Unit (SFU) for scalable real-time WebRTC video and audio streaming. | SFU | no | "selective forwarding unit" |
| ivan-matveev/webrtcbin_ctx | 0 | 2026-01-12 | C++ | Wraps gstreamer webrtcbin to allow synchronious webrtcbin usage. | engine-bindings | no | "gstreamer webrtcbin" |
| jianglu/werift-webrtc-rs-interop-repro | 0 | 2026-09-17 | Rust | Minimal repro: DataChannel never opens between webrtc-rs 0.20.5 and werift 0.24.4 (ICE completes) | other | no | werift |
| k0nserv/str0m-docker-bridge | 0 | 2025-11-17 | Rust | Modified str0m example with docker bridge network | other | no | str0m |
| kaucrow/united-cinemas | 0 | 2025-10-31 | Rust | WebRTC video streaming platform using the SFU architecture. | SFU | no | topic:webrtc-sfu |
| krish9219/openmeet | 0 | 2026-05-11 | JavaScript | Open-source group video calling. A WebRTC SFU (mediasoup) you can read in an afternoon. | SFU | no | topic:webrtc-sfu |
| longjoel/xeyes-webrtc | 0 | 2026-07-02 | Rust | X11 terminal streaming via WebRTC. Launches xterm in Docker with Xvfb, captures and streams through GStreamer  | client-lib | no | "gstreamer webrtcbin" |
| mosamorosev/str0m-e2ee | 0 | 2026-06-24 | C++ | End-to-end encrypted WebRTC conferencing using str0m SFU | SFU | no | str0m |
| ohmygodvt95/gmeet | 0 | 2025-07-11 | Shell | A video conferencing application similar to Google Meet, built with an SFU (Selective Forwarding Unit) archite | SFU | no | "selective forwarding unit" |
| omWAL/interview-system-webrtc | 0 | 2026-01-17 | JavaScript | Web Real-Time Communication  Coding challenges | other | no | webrtc-sys |
| reiver/goldgorilla | 0 | 2026-02-05 | Go | GoldGorilla is bespoke Selective Forwarding Unit (SFU) designed to work with GreatApe. | SFU | no | "selective forwarding unit" |
| ritwikareddykancharla/realtime-sfu-engine | 0 | 2026-02-13 | - | A minimalist Selective Forwarding Unit (SFU) written in Go with Pion WebRTC, implementing Simulcast and adapti | SFU | no | "selective forwarding unit" |
| shinyoshiaki/werift-mediasoup-interop | 0 | 2026-09-08 | TypeScript | Node.js interoperability fixture for mediasoup, mediasoup-client, and werift source polyfill | other | no | werift |
| sjhorn/webrtc_examples | 0 | 2026-01-05 | HTML | webrtc examples in werift and webrtc_dart | client-lib | no | werift |
| soybinastic/mediasoup | 0 | 2026-08-12 | TypeScript | Video conference & WebRTC + SFU (Selective Forwarding Unit) for mini studio app. | SFU | no | "selective forwarding unit" |
| tartavull/webrtc-sys-otto | 0 | 2026-04-19 | Assembly | Minimal webrtc-sys fork for Otto macOS ObjC codec workaround | engine-bindings | no | webrtc-sys |
| vinhlq/esp-webrtc-sys | 0 | 2026-08-02 | Rust | (none) | other | no | webrtc-sys |
| wozhi-cl/webrtc-system | 0 | 2024-07-17 | Vue | webrtc | other | no | webrtc-sys |
| zigouat/sctp | 0 | 2026-09-27 | Zig | SCTP implementation in Zig | engine-bindings | no | webrtc language:zig |
| zyt0929/WebrtcLogSystem | 0 | 2025-09-02 | Python | (none) | other | no | webrtc-sys |

## 6. Sources

- GitHub REST API, unauthenticated: `GET /search/repositories?q=<14 queries>&sort=stars&per_page=30`; `GET /repos/{owner}/{repo}` (existence/status); `GET /repos/{owner}/{repo}/readme` (base64 content) for webrtc-rs/sfu, ionorg/ion-sfu, aiortc. All snapshots 2026-09-28.
- Star/push counts drift; re-verify before citing outside planning docs.
- Rate-limit log: 404s from a curl flag bug (`-d` without `-G` sent POSTs) on 9 queries, re-run as GETs successfully; one genuine transient 403 rate-limit on mediamtx/janus-gateway retried after window reset. Search quotas: `sfu webrtc` 560 total, `webrtc language:rust` 716, `node-webrtc` 2,369, `mediamtx` 594, `janus-gateway` 394 - each truncated at 30.
