# Admin / Dashboard UI Patterns — RTC & Platform Products

> **Status**: frozen research snapshot, 2026-09-28. Do not back-write; new findings go to a new dated file (C2).
> **Method**: live fetch of vendor docs (docs.livekit.io, docs.daily.co, janus.conf.meetecho.com, jitsi.github.io/handbook), GitHub API READMEs (talktome, mumble, FreePBX/framework, ant-design-pro), local read of MediaServo `docs/modules/16-admin-dashboard.md` + decisions ledger. Claims not directly seen in a fetched page are flagged **[UNVERIFIED]**.
> **Scope note**: TalkServo PoC does NOT include admin (PoC = dispatch/field pages). This dossier feeds the Admin surface design section + Alpha backlog.

## 1. Inventory of admin surfaces across references

| Product | Admin surface form | Key objects exposed | Auth model |
|---|---|---|---|
| LiveKit Cloud | Hosted web dashboard | Projects, regions, firewall rules, quotas & limits, billing, Analytics API | Account + per-project API key/secret |
| Daily | Hosted dashboard + REST API | Domain (top-level object), rooms, meetings, participants, tokens, webhooks, recordings, transcripts, logs | Domain API key; meeting tokens |
| Janus | **No official web admin** — Admin/Monitor JSON API only | Sessions, handles, tokens, event handlers, pcap dumps, config setters | `admin_secret` on separate `/admin` path |
| Jitsi Meet | **No admin UI** — config-file-driven deployment (handbook) | `config.js`, `interface_config.js`, nginx, docker env | n/a (SSH + files) |
| talktome | Embedded `/admin` SPA (Node.js + mediasoup + Socket.IO) | Users, feeds, conferences, target order, network config, RTC port range, backups, Guest login, production matrices | Initial `admin` account, forced password change |
| Mumble (murmur) | **No web admin** — IceXML RPC interface (`MumbleServer.ice`) + `murmur.ini` | Live users (mute/deaf/suppress/prioritySpeaker/channel), channels, server settings | Ice secret |
| FreePBX | PHP GUI over modular Asterisk admin (~95 module repos) | Extensions, routes, queues, conferences, IVR, module marketplace | Local admin accounts |

### 1.1 LiveKit Cloud (docs.livekit.io/cloud/)

Fetched "Administration topics" table verbatim covers: **Regions** (latency/redundancy/data-residency), **Firewall configuration** (IP allowlists for rooms), **Quotas & limits** (usage calculation per plan), **Billing**, **Analytics API** (programmatic usage/performance/quality metrics "for building custom dashboards"). Sandbox was removed; replacement tooling is Agent Console + a development token server. Token model (auth docs): `api_key` + `api_secret` signing an `AccessToken` carrying identity + grants; token revocation is Cloud-only. A dashboard **token generator/playground** is widely known but was not seen on a fetched page — **[UNVERIFIED]**.

### 1.2 Daily (docs.daily.co)

REST API index (llms.txt): **Domain** is the top-level object; endpoints for rooms (incl. batch create/delete, livestream start/stop, transcription, dial-out, SIP transfer), meeting tokens (incl. self-signing from API key without a round-trip), webhooks (meeting-started/ended, participant-joined/left, recording-*, transcript-*, dialout-*, waiting-participant-*), recordings, transcripts, logs (call-quality + API logs). Dashboard tour page describes: "manage rooms, inspect sessions, access API keys, and monitor usage" — sessions inspection is a first-class dashboard tab.

### 1.3 Janus (janus.conf.meetecho.com/docs/admin.html)

Official position: demos only, no admin web UI. The Admin/Monitor API is pull-based REST/WS with request families: generic (`info`, `ping`, `get_status`), session (`list_sessions`, `destroy_session`, `accept_new_sessions` — drain mode, `set_session_timeout`), handle/WebRTC (`list_handles`, `handle_info`, `start_pcap`/`stop_pcap`, `hangup_webrtc`, `detach_handle`), tokens, event handlers (`query_eventhandler`, `custom_event`), plus runtime configuration setters. Community panels exist (e.g. `sipwise/janus-admin`, 17 stars, "node.js http client implementing the entire admin interface"). Notable transferable ideas: **drain mode** (`accept_new_sessions=false`), **pcap dump from admin**, **admin secret separate from user auth**.

### 1.4 Jitsi Meet (jitsi.github.io/handbook)

Handbook is a deployment/config guide; administration = editing config files and docker env; no admin web UI is documented. Confirms the "config-driven, zero admin surface" extreme of the design space.

### 1.5 talktome (github.com/thepoison606/talktome README)

Closest structural sibling to TalkServo: local self-hosted WebRTC intercom (mediasoup backend) with an embedded **`/admin`** page on the same HTTPS server as the client app. Admin sections per README: users, feeds, conferences, target order, **network config incl. RTC port range and announced media address**, backups, Guest login, multi-production matrices with scoped users/conferences/feeds and "production admins" (scoped admin roles). First-run flow: seeded `admin` credential → forced password change → create users/conferences → operators use `/`. The Config page can restart the server. Debugging affordance: "if audio does not connect, check Admin Config" — network settings live in admin, not in a file.

### 1.6 Mumble / murmur (github.com/mumble-voip/mumble)

No official web admin. Server control surface is **IceXML/ICE RPC**: `src/murmur/MumbleServer.ice` defines `User` structs with session, mute, deaf, **suppress** (no speech privileges in channel), **prioritySpeaker**, channel, online-secs — i.e. a live floor/state model very close to TalkServo's floor abstraction, exposed as an RPC table. Settings otherwise in `murmur.ini`. Takeaway: even a 2005-era VoIP server separates "live state monitor" (RPC) from "static config" (ini); third-party GUIs (Mumble-Mumble-Web interfaces, unmetered) fill the web-admin gap — **[UNVERIFIED]** (not fetched).

### 1.7 VoIP PBaaS conventions — Asterisk/FreePBX (github.com/FreePBX/framework)

FreePBX = "GUI that controls and manages Asterisk", GPL, PHP, version 14, distributed as ~95 module repos (Core, PBX, Queues, Conference, IVR, etc.). Breadth reference only: the module-per-domain IA (each telephony feature is an installable admin module with its own CRUD page) is the historical pattern behind "admin = list of CRUD pages". Modern RTC platforms (LiveKit/Daily) collapsed this into object-centric nav (project → keys → rooms → sessions → usage).

### 1.8 Ant Design Pro conventions (github.com/ant-design/ant-design-pro README + src/access.ts)

Current stack: React 19 + Umi Max 4 + antd 6, Tailwind v4 + antd-style theming, built-in i18n, mock dev, blocks. Template IA ships Dashboard (Analysis/Monitor/Workplace), Form (basic/step/advanced), List (table/card/search), Profile, Result, Exception (403/404/500), Account Settings, Login/Register. Access control is a declarative map (`src/access.ts` returns `{ canAdmin: currentUser?.access === 'admin' }`, consumed via Umi `access` on routes) — the "canAccess"-style pattern. CRUD convention in Pro blocks is ProTable list + drawer/modal form (ProForm) — **[UNVERIFIED]** at doc level (README shows templates, not the ProTable/ProForm component docs).

### 1.9 MediaServo sister precedent (local, read-only)

`docs/modules/16-admin-dashboard.md` (Phase 4 design): React 19 + Ant Design 5 SPA embedded in the server binary via **rust-embed** (Vite build → `admin-ui/dist/` → compiled in; Gateway serves `/admin/api/*` REST + `/admin/*` SPA fallback + `/health`). Routes: Dashboard, Rooms, RoomDetail, SessionLog, Settings, Login; JWT Bearer auth with 401→login guard. API shapes: `GET /admin/api/dashboard` (active_rooms, connected_peers, cpu_pct, mem_mb), `GET /admin/api/rooms`, `POST /admin/api/auth/login`.
**Tech-choice tradeoff**: the module doc says AntD, but decisions ledger supersedes it — **D196** (2026-07-24): React+TS SPA, rust-embed + build.rs, Zustand frontend state, **Admin JWT separate from signaling JWT**, unified AdminEvent enum for audit + WS push. **D197** (same day): D87 (React+AntD) scope-limited to the *client* GUI; the *server* admin dashboard moved to **CSS Modules zero-dependency** ("monitoring tool, not a user-facing app; don't add Ant Design for a few cards and a table"). The doc and the ledger disagree — treat D196/D197 as current. TalkServo resolves this differently: user fixed React+TS+**AntD** for admin, with zustand state (D196 precedent) and rust-embed serving (D196/MediaServo precedent).

## 2. Recurring admin IA across all references

Consistent object spine: **org/project → API keys/tokens → rooms → live participants → session history → roles/limits → monitoring → logs/audit → server config (network/ports)**. Three tiers recur: (1) *provisioning* (keys, rooms, limits — CRUD lists), (2) *live observability* (sessions, floor state, quality metrics — read-only tables + push updates; Janus pcap and talktome "check Admin Config" show media-level debug belongs here), (3) *account/settings* (auth, billing, backups). Two auth disciplines repeat: admin credentials **separate from user/signaling credentials** (Janus admin_secret, LiveKit key+secret vs client token, MediaServo D196 dual JWT) and first-run forced password change (talktome).

## 3. TalkServo admin v1 candidate surface list

| Surface | Precedent | PoC? / later |
|---|---|---|
| Rooms CRUD-lite (list, create/delete, name, per-room config) | Daily rooms API, talktome conferences, MediaServo `/admin/api/rooms` | **PoC-adjacent** — minimal list+create only if dispatch pages need it; full CRUD later |
| Peers / roles / priority table (live floor state, suppress/priority per user) | Mumble `MumbleServer.ice` User struct (mute/deaf/suppress/prioritySpeaker), LiveKit participant lists | **PoC candidate** — read-only live floor monitor is the highest-value single screen |
| API/JWT keys (key+secret issuance, token playground) | LiveKit api_key/api_secret + AccessToken, Daily self-signed meeting tokens | Later (Alpha) — PoC uses fixed PSK/dev tokens |
| Live monitor (active rooms, connected peers, SFU stats) | MediaServo dashboard API (`active_rooms, connected_peers, cpu_pct, mem_mb`), Daily dashboard usage tab | **PoC candidate** — one status card row + peers table |
| Session/event log + audit (join/leave/floor-grant history) | Daily logs endpoints + meeting list, MediaServo SessionLog route + D196 unified AdminEvent | Later (Alpha) — needs persistence layer first |
| Turn/transport credentials (ICE/UDP port range, announced address) | talktome Admin Config (RTC port range, media address), docker.md UDP 40000-40100 constraint | Later — expose as read-only display in PoC, editable in Alpha |
| Limits config (max rooms/peers/bitrate, quotas) | LiveKit "Quotas & limits", talktome production scoping | Later |
| Drain / accept-new-sessions toggle | Janus `accept_new_sessions` | Later (ops feature, cheap to add once WS server exists) |
| Settings page split (server config vs account/security) | AntD Pro Account Settings template, talktome Config section, FreePBX module split | Later — follow AntD Pro split-menu convention when built |

Implementation assumptions carried from context (user-fixed): React + TS + AntD, zustand state (D196), embedded static SPA served by talkservo-server via rust-embed (MediaServo pattern, D196 build.rs variant). Admin JWT must be separate from signaling JWT (D196 rule, adopt verbatim).

## Sources

- https://docs.livekit.io/cloud/ (Administration overview, fetched 2026-09-28)
- https://docs.livekit.io/home/get-started/authentication/ (tokens/auth, fetched 2026-09-28)
- https://docs.livekit.io/llms.txt + https://docs.livekit.io/cloud/ llms index
- https://docs.daily.co/reference + https://docs.daily.co/llms.txt (domain/rooms/webhooks/logs API map, fetched 2026-09-28)
- https://docs.daily.co/docs/guides/architecture-and-monitoring/experiment-in-the-dashboard.md (via llms.txt summary; page fetch timed out — quoted from index only)
- https://janus.conf.meetecho.com/docs/admin.html (Admin/Monitor API, fetched 2026-09-28)
- https://github.com/sipwise/janus-admin (GitHub search, 2026-09-28)
- https://jitsi.github.io/handbook/ (fetched 2026-09-28)
- https://github.com/thepoison606/talktome README via api.github.com (fetched 2026-09-28)
- https://github.com/mumble-voip/mumble `src/murmur/MumbleServer.ice` via raw.githubusercontent (fetched 2026-09-28)
- https://github.com/FreePBX/framework README via raw.githubusercontent (fetched 2026-09-28)
- https://github.com/ant-design/ant-design-pro README + `src/access.ts` (fetched 2026-09-28)
- LOCAL: /home/maxsense/Documents/ms_rtc/3rdparty/MediaServo/docs/modules/16-admin-dashboard.md
- LOCAL: /home/maxsense/Documents/ms_rtc/3rdparty/MediaServo/.agents/memorys/decisions.md (D196, D197; D87 archived, cited via D197 text)
