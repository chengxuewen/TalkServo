# TalkServo Status

**Generated**: 2026-09-28 | Decisions: 5 entries | Phase: 0 complete, Phase 1 (core crate) not started | Branch: main

> TalkServo is a sister project of MediaServo (same RTC domain). Engineering lessons are accumulated from scratch;
> MediaServo's historical ledgers live in its own repository under `.agents/memorys/` — they are not migrated here and must not impersonate this project's history.

## Project Structure

| Crate / Module | Tests | Notes |
|-------|:----:|------|
| docs/ (whitepaper, architecture, research/) | — | planning drafts v0.1 |

## Phase Status

| Phase | Status |
|-------|:----:|
| 0 Scaffolding/config adaptation | ✅ (docs system ported + English migration; whitepaper/architecture/research generated 2026-09-28) |
| 1+ | ⬜ |

## Decision Status

| Decision | Content | Status |
|------|------|:----:|
| D1 | Floor abstraction unifies PTT/full-duplex/hybrid | ✅ adopted |
| D2 | Centralized signaling + SFU (separate processes) | ✅ adopted |
| D3 | webrtc-rs media engine; no libwebrtc | ✅ adopted |
| D4 | 3A via webrtc-audio-processing FFI (PoC) | ✅ adopted |
| D5 | 4-crate split; default-features discipline | ✅ adopted |

## Next Steps

1. Review/adjudicate docs/architecture.md open questions (OQ-1..5)
2. Establish workspace / pixi / CI skeleton per D5 (`talkservo-core` first)
3. PoC stage 1: Floor state machine + unit tests (whitepaper §9)
