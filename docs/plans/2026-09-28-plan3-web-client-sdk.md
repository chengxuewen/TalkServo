# Plan 3: packages/client (TS SDK) + Web SPA — Task-Level Plan

> Skeleton plan: tasks/interfaces pinned; step expansion at execution start. Prereq: plan-2 server APIs live (wire v:1, modules/02). D14/D9/D12: SDK owns wire/media/state; SPA is thin views; Electron later consumes the same package.

**Goal:** `packages/client` (unpublished) + `web/` SPA passing Playwright e2e against plan-2 server (fake-device, 3 instances), rust-embed into the binary.

**Architecture:** SDK facade `TalkServoClient({role})` over four internal layers (types → signal transport → core state → media manager) per D14 design session diagrams; SPA = dispatcher grid + field page (modules/08), AntD5+zustand views only.

**Revision:** v1.1 (2026-09-28) — +round-2 routed findings section (AudioHandle, token race, connectionstate, micTruth, gen strictness)

**Spec:** docs/modules/08-web-ui.md, docs/modules/02, D12/D13/D14/D16, research/ui/*.

**Global constraints:** C1 English strings; dark theme via ConfigProvider tokens; no react-zdog libs; `setSinkId` matrix per modules/08; iOS Safari in matrix; aria-live floor announcements.

### Task 1: packages/client core
`types.ts` (mirror modules/02; drift CI check vs core JSON-schema once available), signal transport (WS, heartbeat, backoff±jitter, TokenRefresh), wireStore (gen-discard, role-scoped), facade API. Tests: vitest against a mock WS server (sequence J/P/X/R replay fixtures from plan-2 fixtures).
### Task 2: packages/client media manager
mediasoup-client Device/transport lifecycle, produce with `codecOptions:{opusFec:true}`, consumer `<audio>` pool + setSinkId, MediaRestart teardown/rebuild, publish pause awareness (D13: server pauses; client displays mute-state truthfully). Tests: mock mediasoup handler + fixture server.
### Task 3: SPA shell + dispatcher
Vite+React+TS+AntD5 scaffold, routes `/d/:room` `/f/:room`, dispatcher: target grid + tally rings + event log (gen visible here only) + per-line mute/gain + grant-queue strip (FloorQueued live), authorized actions (ModeChange/MuteSet/preempt). Vitest component smoke.
### Task 4: field page + PTT
roster + hold-to-talk (pointer tri-state, Space double-fire guard, keybind capture drawer, release-delay), aria-live, audio cues (grant/deny/taken), iOS AudioContext unlock on Join. Vitest.
### Task 5: Playwright e2e
fake-device 3-instance: full §4 acceptance functional set (hold exclusive, pre-empt, release, reconnect R1, E11 via server-side kill switch endpoint), getStats assertions. Nightly CI job (non-blocking), blocking variant smoke.
### Task 6: rust-embed + gates
`web/dist` embedded in talkservo-server (feature-checked), `pixi run web-build` in CI, gates green, ledger sync.

**Review Focus:** double-tab same identity → SDK surfaces Error{AlreadyJoined} visibly · Safari 12 playback after unlock · queue strip updates only via role-authorized snapshot · keybind conflicts (Ctrl/Cmd+letter rejected) · huge room PeerList render (cap render 50, paginate).

## Round-2 routed findings (land during expansion, D14/D12)

- SDK-owned AudioHandle{pause,release} per consumer; released on FloorIdle/Taken/MediaDown (S-1/#2)
- TokenRefresh cache consumed by reconnect; 4401 → emit `auth-expired`, stop backoff, no auto-retry (S-4/#3)
- `connectionstatechange failed` → local teardown + re-run J3-7 (mirror W-step 3) (S-5b/#12)
- `micTruth = track.enabled && !serverPaused`; UI reads SDK only (S-5/#13)
- gen discard `<` seen; events emitted strictly in gen order per peer (S-6b/#15)
- visibilitychange must not gate media — rule lives in SDK media manager, not views (S-8)
- React owner rule: `useSyncExternalStore` + module-scope client instance + StrictMode-safe cleanup (S-3)
- Dual-gate display: mic truth per D16; role claims `{sub,room,role}` trusted server-side

## plan-review additions (2026-09-28, land during expansion)

- Per-task anchors for routed items: S-3 React owner rule → T3/T4 view code; drift CI gate (schema→TS regen→git diff --exit-code) owned by T1 + T6 gates
- T4 keeps settings-drawer accessibility toggle (modules/08) — was dropped
- T5 e2e: kill-switch endpoint = plan-2 deliverable (dependency noted); T6 rust-embed behind explicit server feature name `embedded-web`
- vitest coverage threshold ≥70% SDK package (SPA views exempt)
