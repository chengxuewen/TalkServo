# Web UI Design
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Screens, layout, visualization (evidence: ui research dossiers)

Two PoC screens, one SPA (`web/`, Vite+React18+TS+AntD5+zustand, rust-embed served — D196-style packaging precedent):

| Route | Role | Layout (normalized from research) |
|-------|------|-----------------------------------|
| `/d/:room` | dispatcher | target grid (tile per peer, tally ring red=holder) + event log side panel (grant/taken/idle + generation shown ONLY here) + per-line listen mute/gain + **grant-queue strip** (industry gap — differentiator) + authorized actions (preempt-broadcast / mute-all / ModeChange) |
| `/f/:room` | field agent | compact live roster (speaking peer highlighted) + one oversized **hold-to-talk** button (pointerdown/up/cancel, thumb zone) + settings drawer (keybind capture rejecting Ctrl/Cmd+letter, release-delay slider, accessibility toggle; `preventDefault` keydown on the HTT to avoid Space double-fire C-5; polite `aria-live` floor announcements C-11) |

Browser matrix (BCD-verified, C-1/C-2/C-3): Chrome/Edge/Firefox/Safari≥12 (mediasoup-client Safari12 handler; Unified Plan only) ⊂ acceptance #9; `setSinkId` all desktop engines, absent on Android browser (field mobile = D8 native); one `<audio>` element per consumer for sink routing; `devicechange` + `replaceTrack` handles headset hotplug.

Floor visualization law: **one state, one source** — on-air is server-event-driven (`FloorGranted/Taken/Idle`), the client never guesses (UI projection of modules/05 §1 principle 2); audio cues on grant/deny/taken (Mumble pattern, non-visual channel).

SPA is a THIN VIEW over `packages/client` (D14) — views + theming + input handling only; all wire/media/state logic lives in the SDK package. Stack decisions fixed: no zod (hand-written TS union mirrors modules/02 §1 until core generates types); no chart libs; dark dispatch theme via ConfigProvider tokens; store split wire/media/ui; iOS Safari AudioContext unlock at Join (gesture-synchronous) is an acceptance item; Admin JWT separate from signaling JWT — rule adopted from D196 now, surface built at Alpha.

**Gap ledger** vs mainstream (full analysis in `reference/research/ui/ptt-ui-patterns.md` §2):

| Class | Items |
|-------|-------|
| parity/lite (8) | tally viz, HTT, per-line mute, cues, roster status, keybind UX, deny feedback, single-transmit control |
| differentiator (1) | grant-queue strip (no surveyed product exposes arbitration state) |
| hard browser gaps (2) | OS-global hotkeys — resolved on desktop by the D10 Electron shell; background/lockscreen receive on mobile — owned by the D8 native SDK track ("native-only acceptance #1") |
| deliberate deferrals | multi-line monitoring / room tree (Alpha; PoC = dispatcher-assigned single room), GPS/SOS workflow, SIP/radio gateway, recording, text chat, admin console surfaces |

## Desktop shell (D10, 2026-09-28)

First-party desktop dispatcher = **Electron shell wrapping `web/` unchanged**: renderer runs the same SPA with embedded Chromium WebRTC (behavior parity with browser acceptance; Discord desktop is the working precedent); main process contributes OS integration only — `globalShortcut` (closes hard gap #5; `register()` returns false when the accelerator is occupied — surface the conflict in settings, review C-4), tray, autostart, `backgroundThrottling:false` hidden-window receive. Shell mechanics (C-7): `requestSingleInstanceLock()`; `render-process-gone → reload()` + R-sequence recovery; Linux auto-update format (AppImage/deb + feed) = Alpha decision; SPA must not gate media on `visibilitychange` (Electron ≥28). No Rust client code in the desktop path; no new UI stack. Tauri evaluated and rejected on WebView media fragmentation; costs accepted (~90 MB, Chromium CVE following, electron-updater in scope). Target: Alpha after PoC web slice; field-agent mobile stays on the D8 native-SDK track.

## Examples & GUI policy (binding demos, not products)

| Lang | Example | GUI | CI role |
|------|---------|-----|---------|
| C | `ptt_send.c` join→request→play→release chain | none | compile + headless smoke |
| C++ | `dispatch_bot.cpp` + `imgui_floor.cpp` (vendored Dear ImGui floor grid; sibling imgui_viewer precedent) | ImGui, compile-only in CI | cmake build + smoke vs stub |
| Python | `floor_monitor.py`, `dispatch_bot.py` (latter doubles as the architecture §4 acceptance probe) | none — headless | run in CI as the automation probe |
| mobile | udl + crate-internal Kotlin smoke | — | unit job |
| Qt | not bundled; headers stay QObject-free (callbacks+POD) so OEM Qt bridging is one-liner | — | — |

Top-level `examples/README.md` will be a one-page index when bindings activate (Beta); no code outside the binding dirs.
