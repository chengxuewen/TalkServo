# Client Design Architecture Review — Web PoC + Electron Shell (D10)

**Snapshot**: 2026-09-28 · **Status**: internal review dossier (frozen per C2) · **Domain**: `internal`
**Method**: read of `modules/02/04/05/08` + `architecture.md` §4 + `research/ui/*`; external fetch of mediasoup-client API docs, Electron docs (app/globalShortcut/webContents/autoUpdater/tray), MDN BCD JSON (raw GitHub), LiveKit docs, W3C ARIA APG. Claims not seen in a fetched source are marked **UNVERIFIED**.
**Scope**: client-facing findings only; server/SFU internals reviewed elsewhere.

## 1. Findings

**C-1 · Safari is in acceptance but the matrix is implicit — doc gap, low severity.**
`modules/08` §1 makes iOS Safari AudioContext unlock an acceptance item, so Safari ⊂ acceptance; `architecture.md` §4 never names browser targets. Evidence: mediasoup-client ships `Safari12` handler (Unified Plan only — no PlanB handler exists in current `BuiltinHandlerName`); `getUserMedia` works Safari 11+ (BCD). FEC: `useinbandfec=1` lives in Router `mediaCodecs` (`modules/04`), negotiated to the browser; but client-side `ProducerCodecOptions.opusFec` default is "browser specific" — pin `codecOptions: { opusFec: true }` on `produce()` and state the tested browser matrix (Chrome/Edge/Firefox/Safari 12+, iOS Safari 12+) in `modules/08`.

**C-2 · "setSinkId is Chrome-only" is stale — reviewer premise corrected by data.**
BCD: Chrome 49, Edge 17, Firefox 116, Safari 18.4 (iOS mirrors 18.4); **chrome_android = false** (Android platform limitation, crbug 41276355). Desktop dispatcher output/headset selection works on every engine; Android *browser* output routing is impossible — acceptable, field mobile is the D8 native track. `MediaDevices.selectAudioOutput` (permission-gated picker) is Chrome/Edge only. Action: record the matrix in `modules/08`; per-line dispatcher audio needs one `<audio>` element per Consumer to route sinks (`modules/04` M1 relay model fits).

**C-3 · Input hotplug mid-session is fully supported — implement, no doc change needed.**
`devicechange` event: Chrome 57 / FF 52 / Safari 11 (BCD). Pattern: listen `devicechange` → `enumerateDevices()` (labels only after mic permission, MDN) → `producer.replaceTrack()` (mediasoup-client API). Not currently mentioned in `modules/08`; one sentence suffices.

**C-4 · PTT keydown collides with browser/OS shortcuts — Chromium-only escape hatch.**
Keyboard Capture API (`navigator.keyboard`, `Keyboard.lock/getLayoutMap`): Chrome 68/69 only; Firefox/Safari unsupported (BCD). In-page `preventDefault` blocks most combos (Ctrl+S/P are preventable) but reserved ones (Ctrl+W/N/T, Ctrl+Shift+Del) cannot be blocked in any browser — **UNVERIFIED** at spec level this round (Chrome reserved-shortcut list not fetched). Electron `globalShortcut.register()` **silently fails if the OS/another app holds the accelerator** (Electron docs) — returns boolean. Actions: (a) field-agent browser path treats the on-screen HTT button (pointer events) as primary, keybind as enhancement; (b) keybind capture rejects Ctrl/Cmd+single-letter combos with a warning; (c) Electron checks `register()` result and surfaces conflict in the settings drawer — `modules/08` gap table claims global hotkeys "resolved on desktop by D10" but the conflict-failure UX is unspecified.

**C-5 · Space activates a focused button — double-fire hazard on the oversized HTT.**
W3C ARIA APG: "Space: activates the button". A keyboard-focused `<button>` fires click on keyup while PTT logic listens keydown → grant+re-release churn. Trivial fix at implementation: `preventDefault()` on keydown for the HTT element or non-button element with pointer handlers. Note for `modules/08` implementation, no design change.

**C-6 · E9 token-expiry UX is silent-vs-visible undefined; wire has no refresh channel.**
`modules/05` E9: "close 4401; client refresh+rejoin" — who triggers refresh, what the agent sees, unspecified. Industry: LiveKit server **proactively pushes refreshed tokens** to connected clients; expiry gates initial connect only; SDK auto-refreshes via `TokenSource` (docs). TalkServo wire (`modules/02`) has no `TokenRefresh` message and no REST refresh endpoint documented → client cannot refresh without a reconnect race. Recommendation: add server→client `TokenRefresh{jwt}` (additive, v:1-safe) or a REST refresh call in `modules/06`; UX = silent refresh+rejoin, visible notice only on refresh failure (field agent mid-shift must not lose floor state beyond R-sequence window).

**C-7 · Electron shell (D10) text omits three required mechanisms.**
(a) **Single instance**: `app.requestSingleInstanceLock()` exists and is the documented pattern; a second launch while the dispatcher is receiving would otherwise fork a duplicate WS+producer session (interacts with C-9). (b) **Renderer crash recovery**: renderer crash ≠ main crash; `app 'render-process-gone'` + `webContents.reload()` in a fresh process is the documented recovery; post-reload the SPA must run the R-sequence (`modules/02` §2) — not mentioned in D10. (c) **Auto-update on Linux**: built-in `autoUpdater` is "only macOS and Windows supported; no built-in support for Linux" (Electron docs); `modules/08` already scopes electron-updater, but the Linux feed/distribution format (AppImage/deb) is undecided — Alpha planning gap, flag now. `backgroundThrottling:false` ✓ present in D10; Electron ≥28 caveat: the flag affects all WebContents in the host window and Page Visibility — the SPA must not gate media on `visibilitychange`. Tray API ✓ documented. Discord listed in the official Electron showcase — D10 precedent holds (blog post itself unreachable this round).

**C-8 · Queue strip leaks arbitration metadata through ServerSnapshot — wire-spec ambiguity.**
`modules/08` puts the grant-queue strip on the dispatcher screen only, and `FloorRequest` is client→server (never broadcast) — live queue is private ✓. But R-sequence delivers `ServerSnapshot{state,gen,peers}` to **every** reconnecting client (`modules/02` §2) and the depth of `state` is undefined: if it carries the pending-request list, any field agent reconnecting learns who queued (sensitive dispatch metadata; queue position = intent-to-speak, a differentiator we chose not to expose publicly). Action: define `ServerSnapshot` payload per role (field: mode+holder+gen+peers; dispatcher: +pending queue) in `modules/02` before coding.

**C-9 · Multi-tab same-identity collision has no server policy.**
`Join{jwt} → Welcome{peerId}` — nothing says what happens when two tabs present the same user token: two peers (roster ambiguity, double producer audio, tally confusion) or reject. LiveKit requires per-participant unique identity and displaces collisions (docs). Recommendation: PoC = reject second Join with `Error{already-joined}` (cheapest, one check in core); displace-mode is an Alpha product decision. Document in `modules/02` §2 J-steps.

**C-10 · i18n: product question, not a defect.**
C1 forces English artifacts; PoC UI strings English-only. Dispatch beachhead customers may be Chinese-market (lead to adjudicate with product owner — outside reviewer scope). Retrofit cost is low (AntD ConfigProvider locale + string extraction at first i18n need); no action for PoC.

**C-11 · Accessibility: audio cues already cover the non-visual channel; add aria-live.**
TeamTalk precedent (`research/ui/ptt-ui-patterns` §1): screen-reader-navigable voice client is a proven design bar. TalkServo has grant/deny/taken audio cues ✓ and toggle-PTT accessibility toggle ✓. Missing: a polite `aria-live` region announcing floor-state changes (one line of markup) so screen-reader users get the same state as sighted users; keep C-5 focus handling in mind when adding it.

**C-12 · PWA / getDisplayMedia — non-findings.**
No service-worker/PWA tier exists in `modules/08`, so iOS-standalone media-capture limits are moot (mobile receive is owned by D8 native track, gap table row 3). `getDisplayMedia` irrelevant: PoC is audio-only. iOS PWA getUserMedia history left **UNVERIFIED** this round (sources unreachable from research env) — revisit only if a PWA tier is ever proposed.

## 2. Verdict

No blocking defect in the client design; D10 + gap-ledger posture matches external evidence. Six doc-precision actions: C-1 (browser matrix), C-6 (token refresh channel), C-7 (Electron mechanisms), C-8 (snapshot scoping), C-9 (collision policy), C-2 (sink matrix). C-4/C-5/C-11 are implementation notes.

## Sources
- mediasoup-client v3 API: https://mediasoup.org/documentation/v3/mediasoup-client/api/ (BuiltinHandlerName, ProducerCodecOptions, ProducerOptions/replaceTrack) — fetched 2026-09-28
- Electron docs: https://www.electronjs.org/docs/latest/api/app (requestSingleInstanceLock, render-process-gone); /api/global-shortcut (silent-fail on taken accelerator); /api/web-contents (backgroundThrottling, forcefullyCrashRenderer+reload); /api/auto-updater ("only macOS and Windows supported"); /api/tray; /apps showcase (Discord) — fetched 2026-09-28
- MDN BCD JSON (raw.githubusercontent.com/mdn/browser-compat-data/main/api/{HTMLMediaElement,MediaDevices,Keyboard,Navigator}.json): setSinkId, selectAudioOutput, devicechange, getUserMedia, Keyboard.lock/getLayoutMap — fetched 2026-09-28
- MDN: https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/setSinkId (security requirements, speaker-selection policy) — fetched 2026-09-28
- LiveKit docs: https://docs.livekit.io/home/get-started/authentication/ (TokenSource auto-refresh); /frontends/reference/tokens-grants/ (proactive refreshed tokens, expiry gates connect only); /home/client/connect/ (auto-resume→ICE-restart→full reconnect with Reconnecting event; unique identity) — fetched 2026-09-28
- W3C ARIA APG: https://www.w3.org/WAI/ARIA/apg/patterns/button/ ("Space: Activates the button") — fetched 2026-09-28
- INTERNAL (read-only): ../../../modules/02-signaling-protocol.md, ../../../modules/04-media-pipeline.md, ../../../modules/05-error-model.md, ../../../modules/08-web-ui.md, ../../../architecture.md §4, ../ui/ptt-ui-patterns.md, ../ui/admin-ui-patterns.md
- NOT reachable this round (claims marked UNVERIFIED): Chrome reserved-shortcuts list; Discord Electron engineering blog (Medium + Wayback 404); MDN PWA compatibility page (no content match)
