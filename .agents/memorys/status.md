# TalkServo Status

**Generated**: 2026-10-10 | Decisions: 21 entries | Phase: plan-4 LAN slice COMPLETE + SIGNOFF (11 pass/1 partial-deferred #8 of 12; D13 consumer reuse fix → keydown p50 ~320ms budget revised 350/600; #9 ICE matrix pass vs LAN coturn; #8 FEC deferred to real impaired network per ruling; public posture = Alpha); P0 multi-web talk LANDED (participant /room shell + Open-concurrency e2e + PIT-21 grant-set rebroadcast fix); Alpha wave-1 = trust & control FULLY ADJUDICATED (D23 + 18-ruling plan-7: audit JSONL / rooms registry / authz chain / mod-claims / tiers / intervention / five-tab; implementation NOT started); web test-hardening batch landed (tautology fix, cross-client asserts, 401-bounce, pw-config parity); plan-8 approved (Momus OKAY) and P1 LANDED (submodule pin 5945f2a, core lane -> ctest -L rust 39 green, pixi flows untouched); P2 LANDED (six lanes: rust x3 + web green via ctest, mediasoup artifact links - conda-cc linker pin, corepack/node22/jsdom toolchain pins recorded modules/09); P3 LANDED (gate.sh ledger-sync + fmt/clippy/deny + e2e-stub/live ctest labels - full table 9/9; lost-globalSetup forensics closed); P4 LANDED (install components bin/dist + talkservo.sh forwarders + CI graph jobs + fresh-clone drill 4/4 - plan-8 COMPLETE) | Branch: main

> TalkServo is a sister project of MediaServo (same RTC domain). Engineering lessons are accumulated from scratch;
> MediaServo's historical ledgers live in its own repository under `.agents/memorys/` — they are not migrated here and must not impersonate this project's history.

## Project Structure

| Crate / Module | Tests | Notes |
|-------|:----:|------|
| packages/client (TS SDK, unpublished D14) | 16 | wire mirror + signal transport + store + facade + media manager; coverage 78.9% |
| web/ (SPA, Vite+React18+AntD5) | 7 | dispatcher + field + participant pages, S-3 session bridge, dark theme; embedded via feature `embedded-web`; e2e: floor/talk/admin specs |
| crates/talkservo-core | 39 | ids + wire contract (27-variant, D16) + FloorState apply() rules 1-9 + grant-set shrink rebroadcast (PIT-21); floor.rs coverage 87.5% |
| crates/talkservo-sfu | 1 | Sfu trait (create/connect/produce/consume/apply_floor diff/peer_left/media_activity E11 seam) + StubSfu call-log backend + MediasoupSfu skeleton (supervisor seam) |
| crates/talkservo-sfu | 1 | Sfu trait + exactly-one-backend compile gate (mediasoup|stub) |
| crates/talkservo-server | 93 | WS signaling (J burst, cooldown, AlreadyJoined, bad_version), media routing (E4/E10), timers (E11/max-hold/TokenRefresh) — stub path; mediasoup worker builds natively (3m52s) |
| docs/ (whitepaper + architecture + modules/01-09 + reference) | — | v0.2 + plan series |

## Phase Status

| Phase | Status |
|-------|:----:|
| 0 Scaffolding/config adaptation | ✅ (docs system ported + English migration; whitepaper/architecture/research generated 2026-09-28) |
| 1 core crate (P0-1) | ✅ 2026-09-29 — workspace + pixi toolchain + talkservo-core domain slice (plan-1 executed) |
| 1+ tooling (plan-6 D19/D20 + B/B2 D21 + C3 routes) | ✅ 2026-10-09 — facade default dir, secrets single-home, front_port yaml-driven, 28 lib tests, live LAN closed loop |

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
| D17 | acceptance = LAN slice; coturn self-hosted | ✅ adopted |
| D18 | admin minimal surface (/admin/api + console) | ✅ adopted |
| D19 | oxmgr supervisor + facade + YAML config (+ Amendment B) | ✅ adopted |
| D20 | default cluster = server + caddy TLS front; --no-web | ✅ adopted |
| D21 | config single source = server.yaml; Caddyfile/oxfile generated | ✅ adopted |
| D22 | admin user/pwd login: accounts.yaml registry (plaintext-or-hash rows, init bootstrap, hot) -> minted JWT | ✅ adopted |
| D25 | PolyOrch build assembly: submodule+CMake graph, scripts thin, runtime red lines | ✅ adopted |

## Next Steps

1. PoC design approved via brainstorm and absorbed into docs/architecture.md v0.2 (dispatch beachhead, mediasoup D6, R1 snapshot-resync, FEC in acceptance)
1b. doc-audit Full run 2026-09-28: 45 findings → 16 merged → all fixed (see memorys/pitfalls.md PIT-3)
2. (done 2026-09-28) arch-review rounds 1+2 applied (D12/D13/D16; 21+19 findings → modules/plans updated; round-2: 21/21 round-1 fixes verified landed)
3. User review of architecture.md + modules v0.2, then implementation plan (workspace/pixi/CI skeleton + talkservo-core first slice)
3. Establish workspace / pixi / CI skeleton per D5+D7 (`talkservo-core` first, P0-1)
3. PoC stage 1: Floor state machine + unit tests (whitepaper §9)
