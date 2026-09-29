# TalkServo Status

**Generated**: 2026-09-29 | Decisions: 16 entries | Phase: 1 core slice LANDED (P0-1) — plan-2 (server+SFU) next | Branch: main

> TalkServo is a sister project of MediaServo (same RTC domain). Engineering lessons are accumulated from scratch;
> MediaServo's historical ledgers live in its own repository under `.agents/memorys/` — they are not migrated here and must not impersonate this project's history.

## Project Structure

| Crate / Module | Tests | Notes |
|-------|:----:|------|
| crates/talkservo-core | 38 | ids + wire contract (27-variant, D16) + FloorState apply() rules 1-9; floor.rs coverage 87.5% |
| crates/talkservo-sfu | 1 | Sfu trait + exactly-one-backend compile gate (mediasoup|stub) |
| crates/talkservo-server | 0 | /healthz axum stub; mediasoup worker builds natively (3m52s first build) |
| docs/ (whitepaper + architecture + modules/01-09 + reference) | — | v0.2 + plan series |

## Phase Status

| Phase | Status |
|-------|:----:|
| 0 Scaffolding/config adaptation | ✅ (docs system ported + English migration; whitepaper/architecture/research generated 2026-09-28) |
| 1 core crate (P0-1) | ✅ 2026-09-29 — workspace + pixi toolchain + talkservo-core domain slice (plan-1 executed) |
| 1+ | ⬜ |

## Decision Status

| Decision | Content | Status |
|------|------|:----:|
| D1 | Floor abstraction unifies PTT/full-duplex/hybrid | ✅ adopted |
| D2 | Centralized signaling + SFU (separate processes) | ✅ adopted |
| D3 | webrtc-rs media engine; no libwebrtc | ✅ adopted |
| D4 | 3A via webrtc-audio-processing FFI (PoC) | ✅ adopted |
| D5 | 4-crate split; default-features discipline | ✅ adopted |
| D6 | SFU = mediasoup crate 0.24 (supersedes D3 engine) | ✅ adopted |
| D7 | PoC scope 3 crates; client deferred | ✅ adopted |
| D8 | SDK strategy: facade+C-ABI base, trigger-grown | ✅ adopted |
| D9 | Web UI scope: 2 screens, admin→Alpha, gap ledger | ✅ adopted |
| D10 | Desktop = Electron shell over web SPA; Tauri rejected | ✅ adopted |
| D11 | pixi + bootstrap, Linux-native-first; Docker=CI-parity | ✅ adopted |
| D12 | FloorState v2 grant-set + MediaDown + queue wire | ✅ adopted |
| D13 | transmission gating (producer pause) + mono | ✅ adopted |
| D14 | TS client SDK package (PoC unreleased; npm at Beta) | ✅ adopted |
| D15 | native engine multi-backend: webrtc-sys/webrtc-rs/stub | ✅ adopted |
| D16 | round-2 hardening: queue purge, peer deltas, wire v/caps, dual-gate | ✅ adopted |

## Next Steps

1. PoC design approved via brainstorm and absorbed into docs/architecture.md v0.2 (dispatch beachhead, mediasoup D6, R1 snapshot-resync, FEC in acceptance)
1b. doc-audit Full run 2026-09-28: 45 findings → 16 merged → all fixed (see memorys/pitfalls.md PIT-3)
2. (done 2026-09-28) arch-review rounds 1+2 applied (D12/D13/D16; 21+19 findings → modules/plans updated; round-2: 21/21 round-1 fixes verified landed)
3. User review of architecture.md + modules v0.2, then implementation plan (workspace/pixi/CI skeleton + talkservo-core first slice)
3. Establish workspace / pixi / CI skeleton per D5+D7 (`talkservo-core` first, P0-1)
3. PoC stage 1: Floor state machine + unit tests (whitepaper §9)
