# Open-Source Voice Infrastructure — Floor-Arbitration Precedents

> Research snapshot: 2026-09-28. GitHub stars/last-push verified per repo via api.github.com on snapshot day
> (raw.githubusercontent.com is blocked from the build network; repo file content fetched via the contents API).
> Docs claims cited to official sites only. Companion to [ptt-landscape.md](../ptt/ptt-landscape.md) (small PTT apps)
> and [media-stack-alternatives.md](media-stack-alternatives.md) (engine choice) — this document covers the
> broad OSS voice-infrastructure field and mines how each system decides **who may speak**.

## 0. Summary matrix

| Project | Verified repo | Stars | Last push | Media model | "Who speaks" mechanism |
|---------|---------------|-------|-----------|-------------|------------------------|
| Mumble | mumble-voip/mumble | 8,303 | 2026-09-26 | Client–server; TCP control (TLS) + UDP voice (OCB-AES128), Opus frames relayed per packet | ACL permission ("Talk") checked by server per channel/link; whisper targets; `prioritySpeaker` state flag |
| LiveKit | livekit/livekit | 21,153 | 2026-09-28 | Go SFU on Pion; forwards track copies untouched; Redis-routed horizontal scaling | Token grants (`canPublish`) = speaking right; `roomAdmin` server API mutes tracks; runtime promote/demote of listeners |
| mediasoup | versatica/mediasoup | 7,377 | 2026-09-25 | TS/Node control plane + C++ libuv workers; pure RTP/DTLS router, no mixing/transcoding | 100% delegated to app signaling; provides `AudioLevelObserver` (volumes) and `ActiveSpeakerObserver` (dominantspeaker) events as floor inputs |
| Janus | meetecho/janus-gateway | 9,170 | 2026-09-23 | C plugin host; AudioBridge **mixes** Opus server-side (libopus); VideoRoom forwards | Self mute/unmute + admin `mute`/`mute_room` (AudioBridge), per-media `moderate` (VideoRoom); `talking` detection via ssrc-audio-level extension (event, not gate) |
| Jitsi Meet | jitsi/jitsi-meet + jitsi/jitsi-videobridge | 30,004 / 3,106 | 2026-09-26 / 2026-09-24 | Kotlin SFU ("multimedia router") + jicofo focus (signaling-only, never touches media) | No hard floor: bridge ranks sources by **speech activity** (dominant speaker, `ConferenceSpeechActivity`), LastN caps video streams per receiver |
| ion-sfu | ionorg/ion-sfu (pion/ion-sfu redirects here) | 1,100 | **2023-07-21** | Go pion-based SFU library; GitHub `archived` flag NOT set | N/A — effectively dead 3+ years; parent `ionorg/ion` is archived |
| Nextcloud Talk | nextcloud/spreed (+ strukturag/nextcloud-spreed-signaling, 534★, 2026-09-23) | 2,203 | 2026-09-28 | P2P mesh ≤5–20 users; High Performance Backend = external SFU + TURN + signaling | Webinar lobby: "only moderators can start/join a call"; raise-hand notifies moderators; Space=PTT, M=mute client UX |
| Rust SFUs | webrtc-rs/sfu 82★ (2026-09-19) · binbat/live777 311★ (2026-09-28) · 8xFF/atm0s-media-server 330★ · restsend/rustrtc 110★ · h3poteto/rheomesh 38★ | — | active | sans-IO state machine / Go-adjacent edge SFU / WHIP-WHEP mesh / WebRTC library / SFU SDK | none implement floor arbitration — all are routers; floor is caller-side |

## 1. Mumble — the closest mature floor precedent

- **Architecture one-liner:** purpose-built voice-chat client/server where the server is an authority over channels, users, and permissions (repo: mumble-voip/mumble; docs: mumble.info).
- **Media path:** TCP control channel (TLS) + UDP voice channel (OCB-AES128, encryption mandatory, UDP may tunnel over TCP); voice is Opus (also Speex/CELT historically) in self-delimiting packets that carry an 8-bit type/target header and optional positional coordinates ([docs/dev/network-protocol/overview.md, voice_data.md](https://github.com/mumble-voip/mumble/tree/master/docs/dev/network-protocol) via contents API). The protocol spec describes per-packet **routing with target resolution** (channel, whisper/shout targets, server loopback); receiver-side spatialization means mixing to speakers happens on the **client** — no server mixer is described in the protocol docs ("server-side mixing" folklore: UNVERIFIED against official docs).
- **Who speaks:** *"Normal talking can be heard by the users of the current channel and all linked channels **as long as the speaker has Talk permission** on these channels"* (voice_data.md). The server enforces an ACL model (inheritance, `@in`/`@all` groups — [mumble.info/documentation/administration/acl/](https://www.mumble.info/documentation/administration/acl/)); channel-viewer state exposes a per-user `prioritySpeaker` boolean ([mumble.info/documentation/developer/channel-viewer-protocol/](https://www.mumble.info/documentation/developer/channel-viewer-protocol/)). Transmission gating (VAD amplitude/SNR, PTT, continuous) is a **client choice**; the server never gates by loudness ([mumble.info/documentation/user/audio-settings/](https://www.mumble.info/documentation/user/audio-settings/)).
- **Maintenance:** 8,303★, pushed 2026-09-26, 486 open issues — the most mature and continuously maintained system in this set.
- **Lesson for TalkServo:** a *permission-checked-at-relay* floor ("server drops VoicePacket if no Talk right") is protocol-simple, decades-proven, and maps 1:1 to our grant/revoke → SSRC relay gating design (§4.2 of architecture.md). Keep loudness heuristics client-side; keep the right server-side.

## 2. LiveKit — grants as an explicit publish-rights model

- **Architecture:** "opinionated, horizontally-scaling WebRTC Selective Forwarding Unit" in Go on Pion; single node has no external deps, Redis added for multi-node room routing ([docs.livekit.io/reference/internals/livekit-sfu.md](https://docs.livekit.io/reference/internals/livekit-sfu.md)).
- **Media path:** SFU forwards "a copy of each stream ... to each interested subscriber **without manipulating any underlying packets**"; audio RED (redundant Opus encoding) is a first-class publish option (`red: false` to disable — [docs.livekit.io/transport/media/advanced.md](https://docs.livekit.io/transport/media/advanced.md)).
- **Who speaks:** capability comes from JWT grants — `canPublish`/`canSubscribe`/`roomAdmin` ([tokens & grants](https://docs.livekit.io/frontends/reference/tokens-grants.md)); the RoomService API mutes a participant's track server-side (`MutePublishedTrack`, requires `roomAdmin`; remote-unmute is a config flag) and `UpdateParticipant` flips permissions live — docs show exactly our promotion pattern: *"promote an audience member to a speaker role ... by granting them the `CanPublish` privilege"* ([participants management](https://docs.livekit.io/intro/basics/rooms-participants-tracks/participants.md), [roomservice-api](https://docs.livekit.io/reference/other/roomservice-api.md)).
- **Maintenance:** 21,153★, pushed 2026-09-28 — very active (AI voice-agent pivot keeps momentum).
- **Lesson:** binary publish-rights *carried in the token and mutable at runtime by an authority* is production-grade floor arbitration without inventing a floor protocol; our FloorToken should behave the same way.

## 3. mediasoup — the "router only" extreme

- **Architecture:** ECMAScript control API + C++ worker subprocess(es) on libuv; ICE/DTLS/SRTP, simulcast/SVC, built-in congestion control ([design](https://mediasoup.org/documentation/v3/mediasoup/design/)).
- **Media path:** pure forwarding — FAQ: no transcoding, no audio mixing, no signaling; everything above RTP is the app ([mediasoup.org/faq/](https://mediasoup.org/faq/)).
- **Who speaks:** not mediasoup's problem — but the worker ships the sensors: `router.createAudioLevelObserver()` emits `volumes`, `ActiveSpeakerObserver` emits `dominantspeaker` ([API docs](https://mediasoup.org/documentation/v3/mediasoup/api/)). talktome (see ptt-landscape.md) proves the talk-lock workflow lives entirely in the control plane on top of these.
- **Maintenance:** 7,377★, pushed 2026-09-25 — active (now under versatica org; `mediasoup-dev/mediasoup` redirects).
- **Lesson:** separating *signal extraction* (level observer) from *arbitration decision* (app) is the right seam; TalkServo SFU should expose volume/talking metrics upward and keep the state machine in `talkservo-core`.

## 4. Janus Gateway — mixer + moderator mute + talking events

- **Architecture:** C "general purpose WebRTC server": core + JSON API over REST/WebSocket/MQTT/etc. + plugins ([index.html](https://janus.conf.meetecho.com/docs/index.html), [rest.html](https://janus.conf.meetecho.com/docs/rest.html)).
- **Media path:** AudioBridge "implement[s] an audio conference bridge ... specifically **mixing Opus streams**" with libopus — a real MCU for audio ([audiobridge.html](https://janus.conf.meetecho.com/docs/audiobridge.html)); VideoRoom is selective forwarding with per-`mid` pause/resume and substream/temporal-layer selection ([videoroom.html](https://janus.conf.meetecho.com/docs/videoroom.html)).
- **Who speaks:** self mute/unmute always mirrors an event to all participants; room admin has `mute`/`unmute` per participant and `mute_room`/`unmute_room`; VideoRoom adds `moderate` per media line. `talking` is derived from the ssrc-audio-level extension (`audiolevel_ext`, thresholds `audio_active_packets`=100/2 s, `audio_level_average`=25) and is broadcast as an event — advisory UI, not a transmit gate.
- **Maintenance:** 9,170★, pushed 2026-09-23 — active; GPL-3.0 (contrast with our Apache/MIT core plan).
- **Lesson:** server-side mute that *announces state changes to everyone* is what makes tally lights and queue UIs work; floor events must be broadcast, not just enforced.

## 5. Jitsi Meet — statistical floor via speech activity

- **Architecture:** jicofo ("focus") runs XMPP signaling and assigns bridges but "does not process any of the media"; jitsi-videobridge is "a WebRTC compatible Selective Forwarding Unit ... a multimedia router" ([jicofo README](https://github.com/jitsi/jicofo), [jvb README](https://github.com/jitsi/jitsi-videobridge) via contents API).
- **Media path:** bandwidth allocation orders sources per receiver: *"starts with sources coming from the endpoints ordered by speech activity (dominant speaker, followed by the previous dominant speaker, etc)"* implemented in `ConferenceSpeechActivity`; **LastN** = max video streams a receiver wants (0 = audio-only survival) ([jvb doc/allocation.md](https://github.com/jitsi/jitsi-videobridge/blob/master/doc/allocation.md)).
- **Who speaks:** nobody is granted the floor — the bridge *ranks* speakers and clients render accordingly; the older handbook pages documenting dominant-speaker signalling are gone from the current site (UI-level claims beyond allocation.md: UNVERIFIED).
- **Maintenance:** jitsi-meet 30,004★ (pushed 2026-09-26), jvb 3,106★ (pushed 2026-09-24) — active.
- **Lesson:** pure statistical arbitration degrades gracefully for hybrid mode (everyone is audible, cameras follow talkers) but cannot *deny* — confirms TalkServo needs explicit grant/deny for PTT and can adopt ranking only for full-duplex UI hints.

## 6. ion-sfu — death report and lessons

- `pion/ion-sfu` returns 301 → `ionorg/ion-sfu`: 1,100★, last push **2023-07-21**, `archived` flag **false** (correction: the "archived" shorthand used in ptt-landscape.md §1 is not GitHub's flag — it is effective abandonment; parent `ionorg/ion` *is* archived).
- **Lessons:** (1) an SFU as a bare Go library without a product/company behind it stalls — webrtc-rs `broadcast`/`rtp-forwarder` examples and ion-sfu remain *architecture references only*; (2) keep `talkservo-core` useful standalone so it cannot die the same way; (3) its room/participant/track API shape is still a clean naming reference.

## 7. Rust-native SFU inventory (2026-09-28, GitHub search-verified)

| Crate/server | Repo (★/activity) | Shape | Relevance |
|---|---|---|---|
| `sfu` | webrtc-rs/sfu (82★, pushed 2026-09-19) | **sans-IO** SFU state machine on the newer `webrtc-rs/rtc` (164★, 2026-09-27): "no sockets, no threads, no clock"; caller owns I/O; `chat` example = WS signaling + UDP | Closest building block: its event/protocol split matches our core↔SFU boundary; but audio/video *router*, zero floor logic (README via API) |
| `live777` | binbat/live777 (311★, pushed 2026-09-28) | "very simple, high performance, edge WebRTC SFU", WHIP/WHEP | Proof Rust SFUs ship daily; watch, don't fork |
| `atm0s-media-server` | 8xFF/atm0s-media-server (330★, pushed 2026-09-25) | Decentralized global-scale mesh media server | P2P routing ideas for later edge mode |
| `rustrtc` | restsend/rustrtc (110★, pushed 2026-09-23) | Alternative full WebRTC stack, claims 2.7× webrtc-rs throughput (author benchmark — unverified) | Possible D3 re-evaluation trigger if webrtc-rs perf bites |
| `rheomesh` | h3poteto/rheomesh (38★, pushed 2026-09-27) | SFU SDK, Rust server + TS client | SDK ergonomics cross-check |
| ~~aurora-sfu~~ | **not found** — 0 GitHub search hits, no crates.io entry | — | Could not be verified to exist; do not cite |
| ~~Kurento (Rust)~~ | Kurento/kurento-media-server 3,053★ **archived**, last push 2023-01-25 | C++/GStreamer media server (MCU-style filters) | A maintained-by-company giant still died; no serious Rust kurento SFU exists |

webrtc-rs `webrtc` crate itself: 5,154★, pushed 2026-09-27; example set re-verified on snapshot day includes `broadcast`, `rtp-forwarder`, `simulcast`, `data-channels-*`, plus new `*-fec` play/save examples — still **no official SFU example**, consistent with media-stack-alternatives.md.

## 8. Comparison — floor arbitration design space

| System | Floor gate location | Gate input | Pre-emption | Deny capability | Server audio processing |
|---|---|---|---|---|---|
| Mumble | server relay, per packet | ACL permission bits | priority flag exists (ducking behavior UNVERIFIED) | yes (no Talk ⇒ dropped) | route/mux, no documented mixing |
| Janus AudioBridge | server mixer | admin mute + self mute | no | yes (`mute_room`) | **mixes** Opus |
| LiveKit | server publish/subscribe policy | JWT grants + admin API | no explicit priority | yes (canPublish=false / mute) | none (router) |
| mediasoup | none (app decides) | observer events (`volumes`, `dominantspeaker`) | app's problem | app's problem | none (router) |
| Jitsi | none — ranking only | speech-activity score | statistical (dominant displaces) | no (everyone audible unless muted) | none (router) |
| Nextcloud Talk | signaling/lobby level | moderator role, lobby state | no | coarse (webinar lobby) | external SFU (HPB) |
| **TalkServo target** | **core state machine + SFU enforcement** | explicit grant/queue/revoke + priority | **yes — barge-in (D1)** | yes | none (router) + FEC intact |

## 9. What TalkServo should steal

1. **Mumble:** check the right *at packet-relay time* (deny = silent drop + notify), never trust client mute state.
2. **LiveKit:** encode floor rights as revocable, runtime-mutable capability tokens (`canPublish`-style FloorToken); support promote-audience→speaker as a first-class admin call.
3. **Janus:** broadcast mute/talking state changes to *all* participants as events (tally lights, queue UI); admin whole-room mute as one request; keep `talking` (ssrc-audio-level) as telemetry only.
4. **mediasoup:** split level-sensing (SFU emits volume/active-speaker events) from arbitration (core decides) — mirrors our signaling-arbitrates / SFU-routes boundary (architecture.md §3.2).
5. **Jitsi:** speech-activity ranking (dominant + recent speakers list, cheap to port) powers full-duplex/hybrid tile layouts without touching the floor.
6. **LiveKit again:** Opus RED as a per-track publish option — cheaper than FEC bookkeeping in our own relay.
7. **ion-sfu/Kurento (negative):** ship a core crate that is useful without the binary, and keep docs self-hosted — both dead projects were single-point-of-maintenance libraries.

## 10. Explicitly UNVERIFIED (kept out of claims)

- Mumble server-side *mixing* (protocol docs show routing + client-side positional mixing).
- Mumble priority-speaker *ducking* behavior (flag verified in channel-viewer state; audio effect not documented on mumble.info).
- LiveKit client-side active-speaker event API details (grant/server-API facts verified instead).
- Talk "only moderators may speak" outside webinar lobby; any Talk 19 LiveKit/Janus backend changelog (changelog greps found nothing on snapshot day).
- rustrtc performance claims (author benchmark).
- Janus AudioBridge silent-packet skipping inside the mixer (levels ⇒ `talking` events verified; mixer CPU-saving internals not in docs).

## 11. Sources

- GitHub REST API `repos/*`, `search/repositories`, `contents/*` (base64), `git/trees` — fetched 2026-09-28. Repos: mumble-voip/mumble, livekit/livekit, versatica/mediasoup, meetecho/janus-gateway, jitsi/jitsi-meet, jitsi/jitsi-videobridge, jitsi/jicofo, ionorg/ion-sfu, pion/webrtc, webrtc-rs/{webrtc,sfu,rtc}, nextcloud/spreed, strukturag/nextcloud-spreed-signaling, binbat/live777, 8xFF/atm0s-media-server, restsend/rustrtc, h3poteto/rheomesh, Kurento/kurento-media-server.
- mumble.info: /documentation/administration/acl/, /user/audio-settings/, /user/faq (negative), /developer/channel-viewer-protocol/.
- docs.livekit.io: reference/internals/livekit-sfu.md, frontends/reference/tokens-grants.md, transport/media/advanced.md, reference/other/roomservice-api.md, intro/basics/.../participants.md (+ llms.txt indexes).
- mediasoup.org: /design/, /faq/, /documentation/v3/mediasoup/api/ (AudioLevelObserver, ActiveSpeakerObserver).
- janus.conf.meetecho.com/docs: index.html, rest.html, audiobridge.html, videoroom.html.
- jitsi: jvb README + doc/allocation.md, jicofo README, nextcloud-talk.readthedocs.io (scalability/, webinar/, call/, index), docs.nextcloud.com user_manual/en/talk/call.html.
- crates.io API: `sfu` (0.5.0, 2026-09-04), `aurora-sfu` → "does not exist".
- Cross-references: [ptt-landscape.md](../ptt/ptt-landscape.md), [media-stack-alternatives.md](media-stack-alternatives.md), [architecture.md](../../../architecture.md).
