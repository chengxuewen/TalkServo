# Web UI Design
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Screens, layout, visualization (evidence: ui research dossiers)

Three role screens + one admin console, one SPA (`web/`, Vite+React18+TS+AntD5+zustand, rust-embed served — MediaServo-sister D196-style packaging precedent):

| Route | Role | Layout (normalized from research) |
|-------|------|-----------------------------------|
| `/dispatch/:room` | dispatcher | target grid (tile per peer, tally ring red=holder) + event log side panel (grant/taken/idle + generation shown ONLY here) + per-line listen mute/gain + **grant-queue strip** (industry gap — differentiator) + authorized actions (preempt-broadcast / mute-all / ModeChange) |
| `/field/:room` | field agent | compact live roster (speaking peer highlighted) + one oversized **hold-to-talk** button (pointerdown/up/cancel, thumb zone) + settings drawer (keybind capture rejecting Ctrl/Cmd+letter, release-delay slider, accessibility toggle, **test transmit (dev)** toggle + sine/clip source selector — the stand-in PTT key from D29 Revision 2: publishes a synthetic track through the NORMAL floor path (request/grant/D13/release all exercised); with the toggle on, getUserMedia is never called; `?tone=1|clip` pre-seeds the source; participant `/room` carries the same control inline; dispatcher is out of scope; `replaceTrack` hot-switch rides the SDK `ProducerLike` surface |

| `/room/:room` | participant (conference shell, P0 2026-10-10) | room header + mode tag + Speakers grid (granted peers as red tags, "(me)" marked) + a single mic **toggle** (requestFloor/releaseFloor — ON AIR only while self ∈ grants, server-driven per the one-state law); rides the field role's session shell, zero protocol additions; multi-client concurrency proven by `web/e2e/talk.spec.ts` |

> **deny ≠ mute (S7.5, 2026-10-20)**: the field and participant surfaces read these two
> states apart, because the wire carries ONE shared reason value for both (`unauthorized`
> — a new variant would be unparsable by pre-S7.5 clients). Standing ban = the peer's own
> id sits in the mirrored `muted` roster: the toggle reads `MUTED BY DISPATCHER`, the
> roster row reads "muted by dispatcher", and the aria-live channel announces the ban.
> One-off refusal = `session.lastDenial` (kept until the floor grants this peer, or until
> the roster lifts the mute it described): `lib/floorCopy.ts` words every `DenyReason` and
> never blanks on an unknown one. When both are true the standing state wins the sentence.
> A refusal also clears the optimistic hold (`holding`/`micOn` key off the event stream, not
> off the reason string — two refusals in a row carry the same value). The dispatcher's mute
> switch is bound to the roster rather than to its own last click; a removal (R7-b, close code
> `4410`) is terminal for the transport and outranks every other line — the button reads
> `REMOVED BY DISPATCHER` instead of promising a retry that will never come (the token still
> validates; only the session ended). And un-mute hands back
> the privilege only. Pinned by `web/tests/denial.test.tsx` (copy) and the
> `e2e/floor.spec.ts` mute row (real browser, real server, live delivery).

Browser matrix (BCD-verified, C-1/C-2/C-3): Chrome/Edge/Firefox/Safari≥12 (mediasoup-client Safari12 handler; Unified Plan only) ⊂ acceptance #9; `setSinkId` all desktop engines, absent on Android browser (field mobile = D8 native); one `<audio>` element per consumer for sink routing; `devicechange` + `replaceTrack` handles headset hotplug.

Floor visualization law: **one state, one source** — on-air is server-event-driven (`FloorGranted/Taken/Idle`), the client never guesses (UI projection of modules/05 §1 principle 2); audio cues on grant/deny/taken (Mumble pattern, non-visual channel).

SPA is a THIN VIEW over `packages/client` (D14) — views + theming + input handling only; all wire/media/state logic lives in the SDK package. Stack decisions fixed: no zod (hand-written TS union mirrors modules/02 §1 until core generates types); no chart libs; dark dispatch theme via ConfigProvider tokens; store split wire/media/ui; iOS Safari AudioContext unlock at Join (gesture-synchronous) is an acceptance item; Admin JWT separate from signaling JWT — rule adopted from MediaServo-sister D196 now, surface built at Alpha.

**Gap ledger** vs mainstream (full analysis in `reference/research/ui/ptt-ui-patterns.md` §2):

| Class | Items |
|-------|-------|
| parity/lite (8) | tally viz, HTT, per-line mute, cues, roster status, keybind UX, deny feedback, single-transmit control |
| differentiator (1) | grant-queue strip (no surveyed product exposes arbitration state) |
| hard browser gaps (2) | OS-global hotkeys — resolved on desktop by the D10 Electron shell; background/lockscreen receive on mobile — owned by the D8 native SDK track ("native-only acceptance #1") |
| deliberate deferrals | multi-line monitoring / room tree (Alpha; PoC = dispatcher-assigned single room), GPS/SOS workflow, SIP/radio gateway, recording, text chat — admin: Monitor/Tokens pulled forward (D18), grown into the wave-1 five-tab trust & control console (D23, §Admin surface); keys/limits/drain UI stay Alpha (the AdminEvent push plane landed server-side as plan-9 W1) |

## Desktop shell (D10, 2026-09-28)

First-party desktop dispatcher = **Electron shell wrapping `web/` unchanged**: renderer runs the same SPA with embedded Chromium WebRTC (behavior parity with browser acceptance; Discord desktop is the working precedent); main process contributes OS integration only — `globalShortcut` (closes hard gap #5; `register()` returns false when the accelerator is occupied — surface the conflict in settings, review C-4), tray, autostart, `backgroundThrottling:false` hidden-window receive. Shell mechanics (C-7): `requestSingleInstanceLock()`; `render-process-gone → reload()` + R-sequence recovery; Linux auto-update format (AppImage/deb + feed) = Alpha decision; SPA must not gate media on `visibilitychange` (Electron ≥28). No Rust client code in the desktop path; no new UI stack. Tauri evaluated and rejected on WebView media fragmentation; costs accepted (~90 MB, Chromium CVE following, electron-updater in scope). Target: Alpha after PoC web slice; field-agent mobile stays on the D8 native-SDK track.

## Admin surface (PoC-tail, D18)

> **wave-1 note (updated 2026-10-20)**: audit log / room registry / command authorization / role tiers / intervention kit + this console are **LANDED** (D23 + docs/plans/2026-10-19-alpha-wave1-trust-control.md, slices S1-S7). Still open: S7.5 (deny≠mute copy on the field/participant pages) and the deferred cross-client admin e2e rows (§Acceptance).

_Landed 2026-10-08 (plan-5 S1/S2): tests `admin_api.rs` + `e2e/admin.spec.ts`. Monitor poll/Tokens link click-through verified._
_Landed 2026-10-20 (plan-7 S1-S7): server lanes `audit_flow` / `authz_flow` / `permission_matrix` / `rooms_registry` / `mod_claims` / `intervention` / `crash_recovery`; web lane `tests/admin.test.tsx` (10 cases) + `e2e/admin.spec.ts`. Test-lane knob `TS_KDF_ITERS` — a debug-build PBKDF2 verify costs ~9 s and starved every browser assertion (PIT-23)._

Minimal admin for LAN operators, grown into the wave-1 trust & control surface: **live floor Monitor + room registry + token issuance + account tiers + intervention kit + audit trail**. Out-of-band REST + the same SPA gains `/admin`; zero additions to the signaling wire, zero media-plane change; fail-closed (`TS_ADMIN_SECRET` unset ⇒ 503).

**Server** (`crates/talkservo-server/src/admin.rs`): `GET /admin/api/overview` (rooms/peers/uptime/backend); `GET /admin/api/rooms` = registry rows (`etc/rooms.yaml`, plan-7 S2: owner/mode/max_peers/max_queue/locked) with `POST /admin/api/rooms` create + `DELETE /admin/api/rooms/{room}` delete; `GET /admin/api/rooms/live` = the poll shape (per-room mode, holders, queue length, peers carrying role/`mod_rooms`/granted/muted, muted set, per-holder speaking activity — speaking cached by the watchdog probe tick, muted from FloorState via the snapshot fix; read via read-only `RoomCommand::AdminInspect(oneshot)` with clone-drop-lock fan-out + 1s per-room timeout stale marking, no shared-state restructure, zero SFU calls on the inspect path); `DELETE /admin/api/rooms/{room}/peers/{peer}` kick + `POST /admin/api/rooms/{room}/peers/{peer}/force-release` (S6 intervention kit — actor/target/reason audited; `reason` is AUDIT-ONLY per R2, never broadcast to peers); `POST /admin/api/tokens` `{sub,room,role,ttl_s,mod_rooms?}` → `{jwt, field_url, dispatch_url, participant_url}` (wraps the existing auth issue; participant link added with the P0 shell, 2026-10-10; `mod_rooms` = moderator claims, S4, additive-default so pre-wave-1 tokens keep verifying); `GET /admin/api/audit` (JSONL tail + event/room/actor filters + cursor, dropped-line counter surfaced, S1); `POST /admin/api/login` plus `GET/POST /admin/api/accounts` and `DELETE /admin/api/accounts/{user}` (D22 registry, S5 tiers). Auth: admin JWT `aud:"admin"` + `role` claim, separate `TS_ADMIN_SECRET` (MediaServo D196 dual-token rule, verbatim); bootstrap = `scripts/issue-admin-token.sh`; 401 without token / wrong audience / wrong secret, no leak; operator-tier writes ⇒ 403 — the server is the real gate, the Sider filter is UX only.

**Web** (`web/src/pages/admin/` — `index.tsx` shell + `monitor.tsx` / `intervention.tsx` / `rooms.tsx` / `tokens.tsx` / `accounts.tsx` / `logs.tsx`, route `/admin` per C3, every file under the plan-7 150-line bar): dark AntD Layout + role-aware Sider **Monitor | Rooms | Tokens | Accounts (admin tier only) | Logs**. Monitor = overview stat cards + rooms table (red tally ring on the holder, reuse `.tally-ring`; adopted plan-9 W2: `/admin/ws` push applies on arrival, and the poll survives as a RECONCILE at 30s while the link is live, tightening to 3s when it is not (the poll is also this tab's only tested error path — R2=A kept it for exactly that reason); a downed link is announced in the same area; a row expands to the peer table where the peer name opens the detail Drawer (role / holding floor / muted / `mod_rooms`, from the live report 6.2) and each row carries `[Force release]` + `[Kick]` behind a confirm Modal with an optional reason input (maxLength 512, audit-only) — nothing fires inline. Rooms = registry CRUD (create modal, delete confirm naming the kick-all consequence); on the operator tier the writes sit behind the visible `registry writes require admin` note; each row carries **Enter** (mints the operator's own dispatch token via the issue endpoint with an OMITTED sub — the server generates `u-xxxxxx` at issue time, D29-#4 — then opens the dispatch join URL) and **Copy links** (all three join URLs, plain-text newline-joined — channel-agnostic paste-to-group, D29-#5); join-failure tiers on the three enter pages: expired (4010 family) = `link expired, ask the operator`, removed (4410) = the operator's reason verbatim (R7), other closes = the ordinary reconnect flow (no apply-to-join affordance — lobby is deferred, D29-#2). Tokens = issue form → copy-ready join links + `mod_rooms` multi-select. Accounts = registry table + create/reset form + role select + delete confirm (registry = `etc/accounts.yaml`, program-maintained, hot). Logs = cursor-paged audit table with event/room/actor filters + dropped-line warning; an `audit_hint` triggers ONE debounced incremental `?after=` fetch per burst (R3=B: a hint never carries a row, so the table keeps a single source of content — the API), and the manual Refresh stays; under W2 an `audit_hint` triggers a debounced incremental refetch and never supplies row content. Gate card first-run (D22 rev2, user/pwd): Username + Password -> `POST /admin/api/login` -> server-minted JWT + role in sessionStorage (tab-scoped, never persisted); no JWT paste in the UI (script mint is CLI-side bootstrap only); any 401/503 from a call bounces to the gate and clears the stored token. No new dependencies.

```text
┌ Sider (tier-aware) ─┐   Monitor │ Rooms │ Tokens │ Accounts(admin) │ Logs
│▸ Monitor            │      ▲         ▲        ▲          ▲                ▲
│  Rooms              │   expand +   CRUD     mod_rooms   role CRUD      cursor +
│  Tokens             │   kick/force modal    multi-select (admin)       filters
│  Accounts (admin)   │   + drawer                                       (manual
│  Logs               │                                                    refresh)
└─────────────────────┘   stat cards: Rooms 3 · Peers 11 · Up 4:12 · Backend stub
```

```text
browser ─/admin─ fetch(Bearer admin-JWT + role) ─► /admin/api/*  [admin.rs]
                                     │ overview · rooms(+live) · tokens · accounts
                                     │ kick · force-release · audit · login
talkservo-server                       ├─ auth: TS_ADMIN_SECRET, aud=admin (fail-closed)
 ├ room task actors ◄─ AdminInspect(oneshot) (read-only) + audited commands (S3/S6)
 ├ audit::Jsonl ──────────────────────► <instance>/logs/audit.jsonl (S1 + dropped count)
 ├ auth::issue ─────────────► user JWT (same HS256 path /ws join verifies)
 └ /ws, /healthz, static embed — untouched; wire unchanged (27 variants)
```

Acceptance (plan-5 + wave-1): missing/wrong admin token ⇒ 401/503 matrix in tests; token issued via API joins /ws successfully; AdminInspect shape stable empty/active; Monitor reflects a live hold within one poll; no admin route reachable without auth (axum integration); operator tier gets 403 on registry/account writes and is refused ModeChange/MuteSet (S3/S5 — `permission_matrix` + `authz_flow`); every admin mutation lands one audit line (S1 `audit_flow`); kick / force-release act on the live room and are audited with actor+target+reason (S6 `intervention`); registry create/delete + lock gate + eviction (S2 `rooms_registry`); `mod_rooms` rides the wire additively (S4 `mod_claims`); kill -9 mid-hold → respawn keeps old tokens valid and rebuilds rooms lazily (S7.6 `crash_recovery`, modules/05 E12). Remaining non-goals (Alpha): keys/limits/drain UI, and the console's adoption of the push plane (plan-9 W2 — the server side landed in W1). The three cross-client browser rows once deferred here (kick-departure, audit line in Logs, locked-room rejection) and the S7.5 deny≠mute copy are all LANDED and green.

## Examples & GUI policy (binding demos, not products)

| Lang | Example | GUI | CI role |
|------|---------|-----|---------|
| C | `ptt_send.c` join→request→play→release chain | none | compile + headless smoke |
| C++ | `dispatch_bot.cpp` + `imgui_floor.cpp` (vendored Dear ImGui floor grid; sibling imgui_viewer precedent) | ImGui, compile-only in CI | cmake build + smoke vs stub |
| Python | `floor_monitor.py`, `dispatch_bot.py` (latter doubles as the architecture §4 acceptance probe) | none — headless | run in CI as the automation probe |
| mobile | udl + crate-internal Kotlin smoke | — | unit job |
| Qt | not bundled; headers stay QObject-free (callbacks+POD) so OEM Qt bridging is one-liner | — | — |

Top-level `examples/README.md` will be a one-page index when bindings activate (Beta); no code outside the binding dirs.
