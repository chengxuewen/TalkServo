# Admin / Dispatch Console Feature Checklist — Mainstream Voice & WebRTC Products

> **Status**: frozen research snapshot, 2026-10-19. Do not back-write; new findings go to a new dated file (C2).
> **Method**: code search over public GitHub (`github.com`, `raw.githubusercontent.com`, code-search index) plus live fetch of `docs.livekit.io`. Every claim below carries the URL it was read from, and a **verification** tag.
> **Scope**: feeds the TalkServo admin console borrow-list. Wave-1 in scope per plan = audit log, room registry, role tiers, kick/force-release, detail drawer. Wave-2 (deferred) = admin audio listen-in, `AdminEvent` WS push.
> **Related**: [admin-ui-patterns.md](admin-ui-patterns.md), [ptt-ui-patterns.md](ptt-ui-patterns.md), [../ptt/commercial-ptt-products.md](../ptt/commercial-ptt-products.md)

## Verification legend

| Tag | Meaning |
|---|---|
| **[V-live]** | Read this session from the live vendor doc |
| **[V-code]** | Read this session from public source code / repo tree |
| **[V-frozen]** | From the frozen 2026-09-28 dossier in this archive; original vendor URL cited but **not re-verified today** |
| **[U]** | Unverified / gap — stated only as a gap, do not build on it |

**Host limitation**: `zello.com`, `motorola.com`, `en.wikipedia.org`, `web.archive.org`, `jitsi.github.io` and `api.github.com` (60 req/h quota exhausted) were **unreachable** from the research host on 2026-10-19. Zello/Motorola items are therefore `[V-frozen]`; Jitsi items were read from the `jitsi/jitsi-meet` source tree instead of the handbook.

---

## 1. Zello Work / Motorola Wave PTX

### Zello / Zello Work — `[V-frozen]`
Source URLs: `https://www.zello.com/`, `https://www.zello.com/pricing/`, `https://en.wikipedia.org/wiki/Zello` (via [ptt-ui-patterns.md](ptt-ui-patterns.md) §1 + [../ptt/commercial-ptt-products.md](../ptt/commercial-ptt-products.md) §1)

- **Management Console in-browser**, one-liner: "Map talk paths — who speaks, listens, or broadcasts — right in your browser"; IT provisions users/channels at scale.
- **Channel is the talk unit**: one-to-many *and* one-to-one; public user-created channels with a **creator → moderator** ownership hierarchy; private channels.
- Console nav items: **Transcription**, **Message History**, **Emergency Alerts**.
- **Radio interconnect**: LMR networks connect to Zello on smart devices; API/SDK access gated to paid tiers.
- Tiers (2026-09-28): Core $8/user/mo, Plus $15, Enterprise adds **on-prem**; first responders free.
- Hardware: earpieces, PTT buttons, speaker mics; Kiosk form factor "Tap, talk. Done."

### Motorola Wave PTX — `[U]`
`motorola.com` / `docs.motorola.com` unreachable (ETIMEDOUT, DNS fail). **No verified feature list this session.** Closest verified commercial dispatch-console precedent in this archive is **MCPTT / Unified Public Safety Network** — `[V-frozen]`, `https://www.mcptt.com/`:
- 3GPP Release 17 MCPTT, **regionalized floor control with carrier-priority access**; Android/iOS share "the same millisecond of floor".
- **Geofenced talkgroups**, per-unit location history, talkgroup stays live through failover.
- **Instant recall from the dispatch console**, multi-select patching, emergency alerts.
- Browser dispatch console is a shipping product, not an add-on.

### PushComm Community — `[V-code]` (closest OSS analog to TalkServo)
Source: `https://github.com/pushcommdigital-gif/pushcomm-community` (AGPL-3.0, pushed 2026-09-01)

- Repo shape: `apps/android-ptt`, `apps/api`, `apps/dashboard`, `apps/dispatch` — **dispatch console and admin dashboard are separate apps**, not pages in one SPA.
- Floor: group channels / all-call / private 1:1, **half-duplex with a server-arbitrated floor** on LiveKit SFU.
- **Dispatch console with floating panels that pop out to a second monitor.**
- **SOS / Lone Worker**: one-tap emergency, **dispatcher acknowledge/resolve**, check-in safety timers.
- **Call recording & CDR**: every transmission captured as a clip, **linked to a call record**, playable + downloadable.
- Users / groups / roles / devices with **IMEI activation, QR provisioning, remote disable**.
- First boot: **dashboard wizard creates the org and the first super-admin account**.
- **Subdomain split**: `dispatch.` / `manage.` / `api.` / `live.`.
- Map: real-time GPS + **follow-the-talker**; geofences/zones/POIs with entry/exit alerts.

---

## 2. LiveKit Cloud dashboard + Jitsi Meet admin

### LiveKit Cloud — `[V-live]`
Sources: `https://docs.livekit.io/cloud/`, `https://docs.livekit.io/intro/cloud.md`, `/deploy/admin/analytics-api.md`, `/deploy/admin/quotas-and-limits.md`, `/deploy/llms.txt`, `/testing/llms.txt`

Dashboard surfaces: **Projects, regions, firewall rules, quotas & limits, billing, Analytics API, Workspaces (Enterprise)**.
"Production-grade analytics, logs, and quality metrics are built into the LiveKit Cloud dashboard" — visibility into **agent behavior, media quality, usage, performance**.

- **Quotas & limits as a first-class registry**: per-project limits on **concurrency, request rates, session durations**. "When a quota is reached, new operations of that kind fail until conditions allow them again" — explicit **fail-closed** semantics. Concurrency/rate limits = stability under load; time/size limits = cap single sessions/recordings/uploads so "a single workload can't consume the platform indefinitely". Metered + included monthly allowance. Free tier: **100 participants**, 2 ingress, 2 egress.
- **Analytics API**: `GET https://cloud-api.livekit.io/api/project/{PROJECT_ID}/sessions/{SESSION_ID}` — programmatic sessions + participants; authorized with an access token carrying the **`roomList` grant** (scope-restricted read, not an admin super-key).
- **Observability split**: built-in session insights *vs* "build your own" via Agents SDK / Egress + **OpenTelemetry trace export** + PII redaction.
- **Log drains**: forward runtime logs to Datadog, CloudWatch, Sentry, New Relic, Splunk, Google Cloud, syslog.
- Region ops: static IPs, **region pinning**, EU data residency.

### Jitsi Meet — `[V-code]`
Source: `https://github.com/jitsi/jitsi-meet` (no vendor web admin; handbook unreachable, see legend)

- **Programmatic console surface** — `modules/API/API.js` dispatches: `grantModerator`, `kickParticipant`, `overwriteParticipantsNames`, `pinParticipant`, `raiseHand`, `sendParticipantToRoom`, `overwrite-config` (whitelisted JSON). → **an admin action set is a flat verb list on one bus.**
- **Kick UX is a 3-file dialog family**: `AbstractKickRemoteParticipantDialog.ts` + `web/` + `native/` variants; `isConfirmDestructive = true`; takes a **reason string** (test passes `'for funzies'`); emits analytics event **`'kick.button'` with `{participant_id}`**; posts a notification naming `kicked` + `kicker` to remaining participants.
- **Permission gating is tested, not assumed**: `kick.spec.ts` asserts `p2.isModerator() === false` before kicking; `mod_muc_kick_participant_spec.js` asserts **HTTP 403 when the `Authorization` header is absent**.
- **Participant detail surfaces** are the `tests/pageobjects/` inventory: `ParticipantsPane`, `Filmstrip`, `Toolbar`, `LobbyScreen`, `Visitors`, `AVModerationMenu`, `SecurityDialog`, `PreMeetingScreen`, `RecordingTranscriptionDialog`, `VideoQualityDialog`, `VirtualBackgroundDialog`, `BreakoutRooms`, `ChatPanel`, `FileSharingPanel`, `InviteDialog`, `PasswordDialog`, `Notifications`, `SettingsDialog`, `LargeVideo`, `IframeAPI`.
- **Role model visible in tests**: moderator / participant / **visitor** (lobby) / **system** (`mintAsapToken`, `mintSystemToken`) — four identities, not two.
- **Third-party admin exists** (vendor ships none): `H2-invent/jitsi-admin` (183★, pushed 2026-10-16) — "Manage and control your video conferences with support for **Jitsi Meet and LiveKit**" + a Matrix bot for creating conferences in a room; `emrahcom/galaxy` (33★) — meeting **schedule + attendees** organizer.

---

## 3. Discord web — channel/role/audit patterns

Source: `https://github.com/discordjs/discord-api-types/blob/main/payloads/v10/auditLog.ts`, `https://github.com/discordjs/discord.js/blob/main/packages/discord.js/src/structures/GuildAuditLogsEntry.js`, `https://github.com/discordjs/discord.js/blob/main/packages/discord.js/src/structures/Guild.js` — all `[V-code]`

- **Audit log is a typed enum, not free text**: `AuditLogEvent` — `GuildUpdate = 1`, `ChannelCreate = 10`, `ChannelUpdate`, `ChannelDelete`, …, `MemberKick`, `IntegrationUpdate`, `InviteCreate`, `AutoModerationRule*`. One number per admin action → the log is filterable and countable by construction.
- **Entry = actor + action_type + target + `changes[]` + `reason`**, where each change is a `{key, old, new}` triple (before/after diff, not just "X happened"). Entry carries `id`, `user_id`, `guild_id`.
- **Filterable retrieval API** — `fetchAuditLogs({ before, after, limit, user, type })`: id-range cursor pagination **plus** filter by **actor** and by **action type**. That 5-field shape is the whole audit-log query surface.
- **Realtime push exists as a first-class event**: `Events.GuildAuditLogEntryCreate` — entries stream in, not only polled. *(Directly relevant to the deferred `AdminEvent` WS push.)*
- **Roles**: separate `permissions.ts` module in every API version (`v6`–`v10`) — permissions are a typed structure, not UI state. `fetchAuditLogs` is **typed on the event** (`GuildAuditLogs<AuditLogEvent.MemberKick>`) so the log narrows by action. — `[V-code]`, role-permission *matrix* itself `[U]` this session.

---

## 4. Mumble web admin over IceRPC

### The RPC contract is the authoritative feature list — `[V-code]`
Source: `https://github.com/mumble-voip/mumble/blob/master/src/murmur/MumbleServer.ice`

**`struct User`** — verbatim fields: `session`, `userid` (−1 = anonymous), `mute`, `deaf` *(implies mute)*, **`suppress`** *(not muted, but **no speech privileges in the current channel**)*, **`prioritySpeaker`**, `selfMute`, `selfDeaf`, `recording` *(read-only)*, `channel`, `name`, **`onlinesecs`**, **`bytespersec`** *(avg bytes/sec over last few seconds)*, `version`/`version2`, `release`, `os`, `osversion`, `identity` *(in-game unique ID)*, `context`, `comment`.

**`struct Channel`**: `id` *(root = 0)*, `name`, `parent` *(−1 = root)*, **`links`** *(linked channels)*, `description`, **`temporary`** *(removed when the last user leaves, read-only)*.

> **`suppress` is the single most borrowable finding in this dossier.** Mumble separates *self-imposed silence* (`mute`) from *privilege-denied silence* (`suppress`), and has a third state (`prioritySpeaker`) that pre-empts both. TalkServo has the floor grant but no surfaced "you have no speaking privilege" channel-state distinction.

### Web UI projects over that contract — `[V-code]`
- **`nycon/mumble-admin`** — `https://github.com/nycon/mumble-admin` (pushed 2026-09-03): modern web UI for **multiple Mumble servers on a bare-metal host**. One-click **install Mumble**; one-click **create server (port + firewall + start automatically)**; start/stop/delete servers at a glance; sidebar server selector; **each server managed independently — users, channels, bans, config**. Dashboard: server status, user count, channel count, **uptime**, **bandwidth**, welcome message. Online users show **status, channel, online time, client info**.
- **`Raddinox/mumble-server`** — `https://github.com/Raddinox/mumble-server` (4★): docker-compose Mumble **with a web frontend & admin page**.

---

## 5. WebRTC SPA testing practice — Jitsi + LiveKit

### Jitsi Meet — `[V-code]`
Sources: `https://github.com/jitsi/jitsi-meet/tree/master/tests` (package.json, wdio configs, helpers, pageobjects, specs), `https://github.com/jitsi/jitsi-meet/tree/master/tests/prosody`

- **Framework: WebdriverIO 9.27**, not Playwright — `@wdio/cli`, `@wdio/globals`, `@wdio/local-runner`, `@wdio/mocha-framework`, `webdriverio@9.27.0`. Node `>=24`.
- **The test harness is an isolated npm project**: `tests/package.json` is deliberately separate from the monorepo so "installing here only pulls in the wdio-side runtime dependencies, not the rest of the monorepo's (webpack/react-native/eslint/...) dependencies." → **one repo-local package = one harness, cheap to run standalone.**
- **Matrix of run configs**: `wdio.conf.ts` (default), `.dev`, `.grid`, `.grid.firefox`, `.firefox`, `.malleus`. Scripts pair every mode with a `-single` variant taking `--spec` for one-file iteration.
- **Load/stress is a separate harness**: **Malleus** (`tests/malleus/cli.ts`, run via `tsx`) — concurrent synthetic participants, not a browser suite.
- **Real XMPP in the loop, not a fake**: `tests/prosody/` runs a real Prosody MUC server; tests assert **protocol-level HTTP status** (`kickParticipant` → `403` when `Authorization` is absent) by minting JWTs (`mintAsapToken`, `mintSystemToken`).
- **Helper layer is domain-shaped**: `Participant.ts`, `participants.ts`, `joinMuc.ts`, `grid.ts`, `token.ts`, `jaas.ts`, `WebhookProxy.ts`, `browserLogger.ts`, `expectations.ts`, `matchers.ts`, `TestsConfig.ts`, `TestProperties.ts`.
- **Reporters**: `@wdio/allure-reporter` + `allure-commandline` + `@wdio/junit-reporter` + `junit-report-builder`.

### LiveKit client-sdk-js — `[V-code]`
Sources: `https://github.com/livekit/client-sdk-js/blob/main/vitest.e2e.config.mts`, `https://github.com/livekit/client-sdk-js/blob/main/smoke-tests/playwright.config.ts`

- **Two-tier test pyramid, three runtimes**:
  - **Unit** — mocked transport, fake DOM.
  - **E2E** — `vitest` **browser mode** (`@vitest/browser-playwright`), `include: ['src/**/*.e2e.test.ts']`, real **Chromium headless**. Comment is explicit: unit tests use a "mocked transport", e2e "exercise[s] the actual browser WebSocket + fetch". `globalSetup` runs in **Node and spawns a live Go mock server** (`../livekit-server/cmd/test-server`); tests run in the **browser**. `fileParallelism: false`, `testTimeout: 20s`, `hookTimeout: 120s`, `screenshotFailures: false`.
  - **Smoke** — separate `smoke-tests/` package, real `@playwright/test`: `fullyParallel: false`, `forbidOnly: !!CI`, `retries: 0`, `reporter: CI ? [['github'], ['list']] : 'list'`, `trace: 'retain-on-failure'`, single `Desktop Chrome` project, and a **`webServer` block that boots the fixture server itself** (`http-server` on `127.0.0.1:4317` with `--cors -c-1`, health-checked against `fixtures/static/esm-direct.html`, `reuseExistingServer: !CI`, `timeout: 30s`).
- **No fake audio device, no screen-capture stubs, no audio assertion at all** — LiveKit's browser suites assert on signalling/subscription state, and leave audio-path verification to the mock server + agent framework.
- **LiveKit's own agent-testing tooling** — `[V-live]`, `https://docs.livekit.io/testing/llms.txt`: pytest + Vitest test framework, **LLM-judged simulations in text and audio modes**, OpenTelemetry trace export, PII redaction, and an **Agent Console** — a browser debugger that observes **live sessions as a hidden participant** (state transitions, conversation + transcription, turn detection, tool execution + results, model usage metrics, errors) with a participants pane exposing **identity, type, attributes, metadata**.

### The transferable norms
1. **Playwright (LiveKit) or WebdriverIO (Jitsi) — never both in one suite.** Jitsi predates Playwright maturity; LiveKit is all-in on Playwright/Vitest.
2. **Serialise the browser suite.** Both set parallelism off. WebRTC signalling tests race on shared server state.
3. **Boot your own fake server in `webServer`/`globalSetup`** rather than pointing at a shared dev env. This is the single most reusable pattern for TalkServo e2e.
4. **Assert at the protocol boundary too** (Jitsi/Prosody 403s) — UI assertions alone cannot prove a permission boundary.
5. **Keep the harness in its own package** so it doesn't drag the app's build graph.
6. **A "hidden observer" role is a legit console feature** — see LiveKit Agent Console's "observe as a hidden participant"; that is exactly TalkServo's deferred admin listen-in, and it is implemented as a *read-only* participant, not an admin bypass.

---

## 6. Deduped recurring console features checklist

Grouped by console area. Each row cites the products that ship it, so the borrow-list can rank by cross-product consensus.

### A. Live room & participant detail *(→ wave-1 detail drawer)*
- [ ] **`suppress` — privilege-denied silence, distinct from `mute`** — Mumble `[V-code]`. *Highest-value single find.*
- [ ] **`prioritySpeaker` pre-emption flag** — Mumble `[V-code]`.
- [ ] **`recording` as a read-only status field** (server-set, not user-set) — Mumble `[V-code]`.
- [ ] **`bytespersec` live bitrate + `onlinesecs` per participant** — Mumble `[V-code]`.
- [ ] **`onlinesecs` as uptime**, alongside **bandwidth** on the dashboard — Mumble + mumble-admin `[V-code]`.
- [ ] **Participant pane exposes identity, type, attributes, metadata** — LiveKit Agent Console `[V-live]`.
- [ ] **Anonymous identity is an explicit value** (`userid = -1`) — Mumble `[V-code]`.
- [ ] **Client version / OS shown per participant** — Mumble `[V-code]`.

### B. Intervention actions *(→ wave-1 kick / force-release)*
- [ ] **Flat verb bus**: `grantModerator`, `kickParticipant`, `overwriteParticipantsNames`, `pinParticipant`, `raiseHand`, `sendParticipantToRoom` — Jitsi `[V-code]`.
- [ ] **Destructive action = explicit confirmation with `isConfirmDestructive`** — Jitsi `[V-code]`.
- [ ] **Kick takes a free-text reason string** — Jitsi `[V-code]`.
- [ ] **Kick emits a typed analytics event with the participant id** (`'kick.button'`) — Jitsi `[V-code]`.
- [ ] **Kick notifies the room with both `kicked` and `kicker` names** — Jitsi `[V-code]`.
- [ ] **`sendParticipantToRoom` — move a participant, not just remove them** — Jitsi `[V-code]`.

### C. Roles & permission boundaries *(→ wave-1 role tiers)*
- [ ] **Four identities, not two**: moderator / participant / **visitor** / **system** — Jitsi `[V-code]`.
- [ ] **`raiseHand` as a permission-gated request, not a chat message** — Jitsi `[V-code]`.
- [ ] **Visitor/lobby role** — held outside the room until admitted — Jitsi `[V-code]`.
- [ ] **Read-only "hidden participant" observer role** — no admin powers, just visibility — LiveKit Agent Console `[V-live]`.
- [ ] **Permission grants are per-read, not per-admin**: read access scoped to `roomList` — LiveKit Analytics API `[V-live]`.
- [ ] **Type the log by event so handlers narrow** (`GuildAuditLogs<AuditLogEvent.MemberKick>`) — Discord `[V-code]`.

### D. Audit log *(→ wave-1 audit page)*
- [ ] **Action = numbered enum, not a string** — Discord `AuditLogEvent` `[V-code]`.
- [ ] **Entry = actor + action + target + `changes[]` + `reason`** — Discord `[V-code]`.
- [ ] **`changes[]` is a `{key, old, new}` diff triple** — Discord `[V-code]`.
- [ ] **`reason` capped at 1–512 chars** — Discord `[V-code]`.
- [ ] **Query = `{before, after, limit, user, type}`** — id-cursor paging + filter by actor + filter by action — Discord `[V-code]`.
- [ ] **Audit entries also stream as a realtime event** (`GuildAuditLogEntryCreate`) — Discord `[V-code]`. *This is the shape TalkServo's deferred `AdminEvent` WS push should take.*

### E. Room / talkgroup registry *(→ wave-1 room CRUD)*
- [ ] **Channel has `parent` + `links` (linked channels), root = id 0** — Mumble `[V-code]`.
- [ ] **`temporary` channels auto-remove when the last user leaves (read-only)** — Mumble `[V-code]`.
- [ ] **Channel `description` surfaced as a tooltip** — Mumble `[V-code]`.
- [ ] **Creator → moderator ownership hierarchy on channels** — Zello `[V-frozen]`.
- [ ] **Geofenced talkgroups + talkgroup survives failover** — MCPTT `[V-frozen]`.
- [ ] **Independent per-server management (users, channels, bans, config)** — mumble-admin `[V-code]`.

### F. Limits & quotas *(→ Alpha)*
- [ ] **Quota registry with explicit fail-closed behaviour** ("new operations fail until conditions allow them again") — LiveKit `[V-live]`.
- [ ] **Separate limit classes**: concurrency / request-rate / session-duration / time-and-size — LiveKit `[V-live]`.
- [ ] **Rationale stated per limit**: stability-under-load vs "no single workload can consume the platform indefinitely" — LiveKit `[V-live]`.
- [ ] **Firewall rules as a console surface** — LiveKit `[V-live]`.

### G. Emergency *(→ Alpha+)*
- [ ] **SOS / Lone Worker with dispatcher acknowledge/resolve** — PushComm `[V-code]`.
- [ ] **Check-in safety timers** — PushComm `[V-code]`.
- [ ] **Instant recall from the dispatch console + multi-select patching** — MCPTT `[V-frozen]`.
- [ ] **Carrier-priority access / regionalized floor control** — MCPTT `[V-frozen]`.

### H. Provisioning *(→ Alpha)*
- [ ] **First-boot wizard creates the org + first super-admin** — PushComm `[V-code]`.
- [ ] **Device activation (IMEI) + QR provisioning + remote disable** — PushComm `[V-code]`.
- [ ] **One-click create server (port + firewall + autostart)** — mumble-admin `[V-code]`.
- [ ] **Multi-server sidebar selector** — mumble-admin `[V-code]`.

### I. Observability & analytics *(→ Alpha)*
- [ ] **Analytics API over sessions + participants, separately authed** — LiveKit `[V-live]`.
- [ ] **Log drains to external sinks** (Datadog, CloudWatch, Sentry, New Relic, Splunk, GCP, syslog) — LiveKit `[V-live]`.
- [ ] **Recording/CDR per transmission, linked to a call record, playable** — PushComm `[V-code]`.
- [ ] **Call-record ↔ media-clip association** — PushComm `[V-code]`.

### J. Console app structure
- [ ] **Dispatch console and admin dashboard are separate apps** — PushComm `[V-code]`.
- [ ] **Subdomain split: `dispatch.` / `manage.` / `api.` / `live.`** — PushComm `[V-code]`.
- [ ] **Floating panels that pop out to a second monitor** — PushComm `[V-code]`.
- [ ] **Separate "talk path map"** — who speaks / listens / broadcasts — Zello `[V-frozen]`.
- [ ] **Schedule + attendees organizer, distinct from the live console** — galaxy `[V-code]`.

### K. Testing *(→ wave-1 e2e)*
- [ ] **Boot your own mock server from the test harness** (`globalSetup` / `webServer`) — LiveKit `[V-code]`.
- [ ] **Serialise the browser suite** (`fileParallelism: false`, `fullyParallel: false`) — both `[V-code]`.
- [ ] **`retries: 0` + `forbidOnly: !!CI`** — LiveKit `[V-code]`.
- [ ] **`trace: 'retain-on-failure'`, reporter = CI-aware** — LiveKit `[V-code]`.
- [ ] **Harness in its own package, decoupled from the app build graph** — Jitsi `[V-code]`.
- [ ] **Protocol-boundary assertions** (HTTP 403 for missing auth) alongside UI assertions — Jitsi `[V-code]`.
- [ ] **Separate load harness** (Malleus), not folded into browser tests — Jitsi `[V-code]`.
- [ ] **No fake audio devices, no audio-content assertions** — LiveKit `[V-code]`.

---

## 7. What nobody ships *(negative space — do not build)*
- **A vendor-built web admin for Jitsi does not exist.** Two third parties built it (183★ / 33★) and both target **both** Jitsi and LiveKit. Evidence that the vendor console is a durable gap, not an oversight. `[V-code]`
- **Nobody surfaces per-transmission talk-group recording to the dispatch console except the OSS PushComm build.** Every commercial product stops at "Emergency Alerts". `[V-frozen]` + `[V-code]`
- **No product separates "you were denied the floor" from "you muted yourself" in the UI** except Mumble's RPC `suppress`. TalkServo is in a position to do this for free, because the floor decision is already server-side. `[V-code]`

## 8. Open items for the borrow-list pass
- Motorola Wave PTX: `[U]` — needs a reachable source. Retry from a host with `motorola.com` egress.
- Discord role-permission **matrix** (channel overrides, hoist/singleton): `[U]` this session; `permissions.ts` confirmed to exist in every API version.
- LiveKit `Analytics API` full field list: endpoint + auth grant verified, response schema not captured.
