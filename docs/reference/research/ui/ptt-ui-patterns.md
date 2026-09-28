# PTT / Voice / Dispatch Product UI Patterns — Research Dossier

**Snapshot**: 2026-09-28 · **Status**: archived (frozen per C2) · **Domain**: `ui`
**Method**: text-only sources — official docs, product sites, GitHub READMEs via `api.github.com`, Wikipedia, one Wayback snapshot. No screenshots (no browser in research env). Anything not readable from a fetched source is marked **UNVERIFIED**, never guessed.
**Consumer**: TalkServo Web UI design section. PoC = browser vertical slice, roles `dispatcher` / `field` via `?role=`, TS+React+AntD; floor semantics (Exclusive/Open/Hybrid, grant/taken/idle events, generation) per [`../../../architecture.md`](../../../architecture.md).

## 1. Per-product patterns

### Discord (consumer gaming voice)
- IA: a "server" is a collection of persistent chat rooms and **voice channels** — channel rail as primary navigation ([Wikipedia](https://en.wikipedia.org/wiki/Discord_(software))).
- Exactly two input modes, chosen in User Settings → Voice & Video: **Voice Activity** vs **Push-to-Talk** ([support art. 211376518](https://support.discord.com/hc/en-us/articles/211376518-Voice-Input-Modes-101-Push-to-Talk-Voice-Activated), read via [Wayback 2024-02-28 snapshot](https://web.archive.org/web/20240228040302/https://support.discord.com/hc/en-us/articles/211376518-Voice-Input-Modes-101-Push-to-Talk-Voice-Activated)).
- Voice Activity tuning = one sensitivity bar + "auto-determine" toggle; docs concede an **inherent ~200 ms activation delay** (same article) — VA is a weak fit for floor arbitration.
- PTT keybind config UX: "click on the shortcut box, and press your desired PTT key" — press-and-hold semantics, capture-on-focus widget (same article).
- **PTT Release Delay slider**: tail time after key release before transmission cuts ("door closing" metaphor) — explicit floor-release latency knob (same article).
- Speaking-user highlight (member-list ring): widely reported but not readable in fetched sources — **UNVERIFIED**.
- Help-center nav lists a "Mobile Voice Overlay (Android)" article — overlay-as-PTT surface exists; details **UNVERIFIED**.

### Zello / Zello Work (commercial PTT, 5M MAU claimed)
- Core paradigm: "Push a button. Talk instantly." — one-button walkie-talkie emulation over data networks ([zello.com](https://zello.com/), [Wikipedia](https://en.wikipedia.org/wiki/Zello)).
- Channels are the unit of talk: one-to-many and one-to-one, marketed as "flexible channels… never cluttered" (zello.com).
- Audio plays "in real time no matter what's on the screen" — receive path is decoupled from foreground UI (zello.com).
- Hardware ergonomics: earpieces, **button accessories**, speaker mics; Kiosk form factor "Tap, talk. Done." (zello.com).
- Zello **Management Console**: "Map talk paths — who speaks, listens, or broadcasts — right in your browser"; IT configures users/channels at scale (zello.com) — dispatch-console-as-web-admin pattern.
- Enterprise feature nav: Transcription, Message History, Emergency Alerts (zello.com).
- The literal app layout (channel tiles, big bottom PTT bar) is only inferable from marketing copy — exact layout **UNVERIFIED**.

### Mumble (OSS gaming VoIP, murmur protocol)
- IA: root channel + **hierarchical channel tree** with users as leaves; "linked channels" merge adjacent channels for announcements (e.g. a small huddle listening to a common channel) ([Wikipedia](https://en.wikipedia.org/wiki/Mumble_(software))).
- In-game **overlay renders connected and/or talking participants** — talking state drawn as a persistent ambient layer, not a settings screen ([mumble.info overlay docs](https://www.mumble.info/documentation/user/in-game-overlay/)).
- **Audio cues**: sounds on transmit start/stop; a "mute cue" when you try to talk while muted; idle-action auto-mute ([mumble.info audio settings](https://www.mumble.info/documentation/user/audio-settings/)) — floor state broadcast non-visually.
- **Global shortcuts**: PTT bound as OS-level hotkey (incl. controller buttons) ([mumble.info global shortcuts](https://www.mumble.info/documentation/user/global-shortcuts/)).
- Simple server admin UI, low-latency emphasis (Wikipedia).

### TeamTalk (OSS-adjacent freeware conferencing, BearWare.dk)
- Server + client conferencing (audio + VP8 video), cross-platform incl. iOS/Android ([Wikipedia](https://en.wikipedia.org/wiki/TeamTalk), [bearware.dk](https://www.bearware.dk/)).
- Distinctive: majority user base is **visually impaired**; client deliberately uses only standard window controls so screen readers navigate it — accessibility-first discipline, a rare documented UI constraint (Wikipedia).
- Channel tree + per-user status detail in current client: not readable from fetched sources — **UNVERIFIED** (Qt client repo: [BearWare/TeamTalk5](https://github.com/BearWare/TeamTalk5)).

### talktome ([thepoison606/talktome](https://github.com/thepoison606/talktome), browser intercom on Node+mediasoup+Socket.IO)
- Operator UI: **direct targets** (per-person buttons), **conferences**, **Reply**, and **Talk Lock** — four named primitives of the talk surface (README Features).
- **Camera tally as floor light**: external `/cut-camera` API turns a user's UI tile **red (pgm/on-air)** or **green (prv)** — server-driven binary "who's live" affordance, borrowed from broadcast galleries (README Camera Tally).
- **Feeds**: listen-only participants publish program audio with per-feed **volume and mute** controls and *cannot talk back* — clean receive-role separation (README Users/Feeds).
- Admin UI is separate from live UI: users, feeds, conferences, **target order**, network/RTC ports, backups, Guest login (README).
- Guests: passwordless shareable login URL + **QR code**, choose own display name, still answerable via Reply (README).
- Multi-production matrices with scoped users/conferences/production admins (README) — enterprise scaling, beyond PoC.

### Gryt ([gryt.chat](https://gryt.chat), [Gryt-chat/gryt](https://github.com/Gryt-chat/gryt))
- Discord-shaped product: self-made servers, voice/video/text, web app at app.gryt.chat + desktop; README preview alt-text describes "a voice call with six people, a chat channel" — call grid + chat side-by-side.
- **Global push-to-talk with configurable OS-level keybinds**; RNNoise suppression; Opus (README Features).
- No-signup identity, server creation from the client, self-host via Docker Compose, mDNS LAN discovery (README + site) — low-friction join UX.

### Eyevinn intercom-frontend ([repo](https://github.com/Eyevinn/intercom-frontend))
- "Low latency, web based, open source… voice-over-ip intercom" client (Vite/React) paired with a separate `intercom-manager` backend; config via env (`VITE_BACKEND_URL`), OSAAS-hosted variant (README).
- UI layout/screenshots: README carries none — **UNVERIFIED**; value here is the client/manager split, not visuals.

### openPTT TRX ([harrowiersma/PTT](https://github.com/harrowiersma/PTT), self-hosted PTT server: GPS, SOS, weather ATIS, fleet dispatch)
- Dashboard IA (operator manual, [`server/dashboard/help.html`](https://github.com/harrowiersma/PTT/blob/main/server/dashboard/help.html)): three groups — **Live Ops** (Overview, Dispatch: "what's happening right now"), **Directory** (Users, Channels: fleet roster), **System** (SIP Gateway, Dispatch Setup, Features, Admins, Audit Log, Call Log: config + history).
- **Feature-gated UI**: each module hides itself when its feature is disabled in System → Features.
- Special channels: **Emergency** (auto-created; *all users moved into it during SOS*), **Weather** (double-press PTT), **Phone** (SIP inbound) — floor events restructure channel membership.
- Hardware PTT gestures on Hytera P50: press-to-talk/release-to-listen; **triple-tap PTT** = lone-worker toggle; **double-press** = weather; orange key cycles presence — one button, chorded secondary semantics.
- **Call Groups**: restrict channels to member subsets; Layer 1 "bounce on entry", Layer 2 "hide from channel tree" toggle (README-less repo; help.html).
- **Pending pills** in member modal: per-user chips `pending cert` / `pending registration` / `registered` — enrollment state as list badges.
- Dispatch map is Leaflet-based (dashboard ships leaflet.css/js + marker icons; repo tree).

### EVO-PTT ([Theofilos-Chamalis/EVO-PTT](https://github.com/Theofilos-Chamalis/EVO-PTT), [evoptt.com](https://evoptt.com))
- Two-surface split, stated as a table: **Android app** for field workers (Voice PTT, GPS, SOS, hardware PTT button on F22/F22+/F25 devices) vs **Web dashboard** for company managers (Users, Companies, **live map**, **server status**) (README "What's in the box").
- Dashboard screen list readable from screenshot alt-texts: Login / Server status / Companies / Managers / Users / User location (README Screenshots).
- Per-company Docker isolation, hosted demo at evoptt.com; repo is a commercial showroom (closed voice server) — matches our own ptt-landscape.md verdict.

### Commercial dispatch consoles (Novacap, Zello Work)
- Zello Work: covered by Zello Management Console claims above (zello.com).
- Novacap: `novacap.com` unreachable from research env (connection refused) — all Novacap console UI claims **UNVERIFIED**; no details invented here.
- Generic TETRA-style console conventions (line priority, patch panels): **UNVERIFIED** in this snapshot; see `../ptt/standards-ptt-mcptt.md` for protocol-level semantics instead.

## 2. Synthesis

### (a) Recurring information architecture
| Surface | Seen in | TalkServo relevance |
|---|---|---|
| Channel/team list as primary rail | Discord, Mumble, openPTT Directory, Zello | dispatcher channel picker |
| Member list with live per-user state | Mumble tree, openPTT pills, talktome targets | floor-holder highlight |
| One big transmit control | Zello, openPTT hardware, EVO hardware | field-agent PTT button |
| Live-ops overview vs directory vs system | openPTT 3-group split | dispatcher IA skeleton |
| Map pane (fleet location) | openPTT Leaflet, EVO user location | post-PoC (no GPS in scope) |
| Separate admin/config surface | talktome, Zello console, EVO dashboard, openPTT System | keep out of live screens |
| Event/audit log | openPTT Audit + Call Log | floor-event log (grant/taken/idle + generation) |
| Listen-only role with per-line gain/mute | talktome feeds | monitor-only field variant |

### (b) Floor-state ("who speaks now") visualization — the core screen problem
- **Tally lamp** (talktome): server-pushed red/green tile state = on-air binary; maps 1:1 to `taken` (red ring on speaker tile) vs `idle`.
- **Ambient talking indicators** (Mumble overlay, Discord UNVERIFIED): highlight follows the voice without a dedicated panel — for TalkServo: animated ring on the speaking user's target tile + channel row.
- **Audio cues** (Mumble): transmit-on/off sounds + mute-denied cue — cheapest non-visual floor feedback; grant-denied blip for contested Exclusive floors.
- **State chips** (openPTT pending pills): compact per-user status badges; reuse for `grant` (pending) / `taken` (active) / `idle`, with `generation` shown only in event log, never in the live grid.
- **Channel restructure as signal** (openPTT Emergency move-all): a floor event that visibly changes the roster (priority preemption) reads louder than any badge.
- No product fetched shows an explicit floor-arbitration countdown/queue UI — differentiation gap for TalkServo dispatcher (grant queue strip).

### (c) PTT affordance options
- **Hold-to-talk** is universal for hardware PTT (Discord "press and hold", openPTT press/release, EVO device buttons). Toggle-style PTT appears nowhere in fetched docs — default to hold, add toggle only for accessibility.
- **Keybind capture UX** (Discord): click box → press key; store as string; plus **release-delay knob** (Discord) — adopt both; our equivalent: keyboard key as virtual PTT button, release-delay = floor-release tail config.
- **OS-global hotkey** (Mumble global shortcuts, Gryt keybinds): browser can't do this — PoC limitation; document, consider later PWA/keyboard-capture API.
- **On-screen big button** (Zello paradigm): thumb-zone bottom bar for touch; exact Zello layout UNVERIFIED but the one-button paradigm is sourced marketing.
- **Chorded gestures on one button** (openPTT double/triple-tap): powerful on hardware, risky on screen (disambiguation latency) — skip for PoC.
- Voice-activation as an input mode (Discord VA): ~200 ms delay, noisy tuning — reject for Exclusive floors; acceptable only for Open-floor "hot mic" experiments.

### (d) Multi-line monitoring conventions
- **Per-line mute/volume** (talktome feeds): real ducking control per monitored source — dispatcher per-channel listen toggle.
- **Linked/announcement channels** (Mumble): priority audio injected into a huddle — precedent for all-call/preempt presentation.
- **Scoped visibility** (openPTT Call Groups hide-from-tree; talktome production scoping): show dispatcher only monitored channels, never the full world.
- **Talk-path map** (Zello console "who speaks, listens, broadcasts"): dispatcher overview = relationship view, not just per-channel view.
- **Auto-select-last-talker**: found in no fetched source — **UNVERIFIED** as a convention; do not implement on evidence.

### (e) TalkServo dispatch console: adopt / skip
**Dispatcher (PoC)**: adopt openPTT 3-group IA (Live Ops / Directory-lite / Settings) with Overview stat cards; speaker **tally ring** (red = taken) on target tiles (talktome); per-user state chips grant/taken/idle (Mumble cues + openPTT pills); per-channel listen mute (talktome); floor-event log with generation numbers (openPTT audit precedent); grant-queue strip (gap identified in (b)). Skip: multi-production matrices, SIP gateway, lone-worker, kiosk, AI transcription, map pane (no GPS in PoC scope).
**Field agent (PoC)**: adopt single-channel focus + one oversized hold-to-talk button bottom-center (Zello paradigm); Discord-style keybind capture + release-delay slider in a small settings drawer; speaking-peer highlight in a compact roster (Mumble); denied-grant audio cue (Mumble mute-cue pattern). Skip: channel tree browsing (assignment comes from dispatcher), camera tally semantics, guest QR (post-PoC), VA input mode.
**Both**: keep admin/config off live screens (universal pattern); one visible "on-air" state, server-driven, never client-guessed.

## Sources
- Discord: [Wikipedia — Discord (software)](https://en.wikipedia.org/wiki/Discord_(software)); [support — Voice Input Modes 101](https://support.discord.com/hc/en-us/articles/211376518-Voice-Input-Modes-101-Push-to-Talk-Voice-Activated) (via [Wayback snapshot 2024-02-28](https://web.archive.org/web/20240228040302/https://support.discord.com/hc/en-us/articles/211376518-Voice-Input-Modes-101-Push-to-Talk-Voice-Activated); live URL returned 404/JS-challenge in research env)
- Zello: [zello.com](https://zello.com/); [Wikipedia — Zello](https://en.wikipedia.org/wiki/Zello)
- Mumble: [in-game overlay docs](https://www.mumble.info/documentation/user/in-game-overlay/); [audio settings docs](https://www.mumble.info/documentation/user/audio-settings/); [global shortcuts docs](https://www.mumble.info/documentation/user/global-shortcuts/); [Wikipedia — Mumble (software)](https://en.wikipedia.org/wiki/Mumble_(software))
- TeamTalk: [Wikipedia — TeamTalk](https://en.wikipedia.org/wiki/TeamTalk); [bearware.dk](https://www.bearware.dk/); [BearWare/TeamTalk5](https://github.com/BearWare/TeamTalk5)
- talktome: [README via api.github.com](https://github.com/thepoison606/talktome)
- Gryt: [gryt.chat](https://gryt.chat); [Gryt-chat/gryt README](https://github.com/Gryt-chat/gryt)
- Eyevinn: [intercom-frontend README](https://github.com/Eyevinn/intercom-frontend)
- openPTT TRX: [repo](https://github.com/harrowiersma/PTT); [server/dashboard/help.html](https://github.com/harrowiersma/PTT/blob/main/server/dashboard/help.html) (no root README — 404 from readme API)
- EVO-PTT: [Theofilos-Chamalis/EVO-PTT README](https://github.com/Theofilos-Chamalis/EVO-PTT); [evoptt.com](https://evoptt.com)
- Novacap: [novacap.com](https://www.novacap.com/) — unreachable (fetch refused); content UNVERIFIED
- Cross-refs (read-only): [`../../../architecture.md`](../../../architecture.md), [`../ptt/ptt-landscape.md`](../ptt/ptt-landscape.md), [`../ptt/standards-ptt-mcptt.md`](../ptt/standards-ptt-mcptt.md)
