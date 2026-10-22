# TalkServo Status

**Generated**: 2026-10-10 | Decisions: 27 entries | Conventions: 5 (C1-C5) | Phase: plan-4 LAN slice COMPLETE + SIGNOFF (11 pass/1 partial-deferred #8 of 12; D13 consumer reuse fix → keydown p50 ~320ms budget revised 350/600; #9 ICE matrix pass vs LAN coturn; #8 FEC deferred to real impaired network per ruling; public posture = Alpha); P0 multi-web talk LANDED (participant /room shell + Open-concurrency e2e + PIT-21 grant-set rebroadcast fix); Alpha wave-1 = trust & control S1-S7 LANDED 2026-10-20 (D23 + 18-ruling plan-7: audit JSONL / rooms registry / authz chain / mod-claims / tiers / intervention kit / five-tab admin console — vitest 18/18, stub-lane e2e incl. admin click-through + the live mute row, PIT-23 TS_KDF_ITERS lane knob); S7.5 deny≠mute LANDED as the A1/A2/A3 walkthrough (mute really revokes the seat and the queue slot, any room member can be muted, un-mute returns the privilege only; the wire keeps ONE shared `unauthorized` reason for D16 compat) — wave-1 CLOSED; the three deferred cross-client admin e2e rows LANDED 2026-10-21 — landing them found the RoomsTab create bug (typed values never reached the POST), the harness gap that made S1/S2 unreachable from a browser (audit_log/rooms_file derive from a config FILE, the lane runs on env) and a CI lane that self-skipped the whole admin spec (no TS_ADMIN); PIT-26 records one unreproduced flake; web test-hardening batch landed (tautology fix, cross-client asserts, 401-bounce, pw-config parity); lesson-review batch closed the loop: **C4 (ledger-index consistency) is now gate-enforced** via scripts/gate.sh + gate-gen.py (mutation-tested), plus PIT-27..29, two browser-lane coverage rules in testing.md and edit-safety rule 24; web test-hardening batch landed (tautology fix, cross-client asserts, 401-bounce, pw-config parity); plan-8 approved (Momus OKAY) and P1 LANDED (submodule pin 5945f2a, core lane -> ctest -L rust 39 green, pixi flows untouched); P2 LANDED (six lanes: rust x3 + web green via ctest, mediasoup artifact links - conda-cc linker pin, corepack/node22/jsdom toolchain pins recorded modules/09); P3 LANDED (gate.sh ledger-sync + fmt/clippy/deny + e2e-stub/live ctest labels - full table 9/9; lost-globalSetup forensics closed); P4 LANDED (install components bin/dist + talkservo.sh forwarders + CI graph jobs + fresh-clone drill 4/4 - plan-8 COMPLETE) | Branch: main

> TalkServo is a sister project of MediaServo (same RTC domain). Engineering lessons are accumulated from scratch;
> MediaServo's historical ledgers live in its own repository under `.agents/memorys/` — they are not migrated here and must not impersonate this project's history.

## Project Structure

| Crate / Module | Tests | Notes |
|-------|:----:|------|
| packages/client (TS SDK, unpublished D14) | 24 | wire mirror + signal transport + store + facade + media manager; coverage 78.9% |
| web/ (SPA, Vite+React18+AntD5) | 25 | dispatcher + field + participant pages (deny≠mute copy via lib/floorCopy.ts, plan-7 S7.5), admin five-tab console (pages/admin/, S7), S-3 session bridge, dark theme; embedded via feature `embedded-web`; e2e 15 tests/5 files in the graph lane (floor incl. the live mute row; admin: click-through + cross-client kick + audit Logs line + registry lock) |
| crates/talkservo-core | 42 | ids + wire contract (27-variant, D16) + FloorState apply() rules 1-9 (Rule 2/8 re-scoped by S7.5: mute revokes grant + queue slot) + grant-set shrink rebroadcast (PIT-21); floor.rs coverage 87.5% |
| crates/talkservo-sfu | 1 | Sfu trait (create/connect/produce/consume/apply_floor diff/peer_left/media_activity E11 seam) + StubSfu call-log backend + MediasoupSfu skeleton (supervisor seam) |
| crates/talkservo-sfu | 1 | Sfu trait + exactly-one-backend compile gate (mediasoup\|stub) |
| crates/talkservo-server | 88 (stub lane) | WS signaling (J burst, cooldown, AlreadyJoined, bad_version), media routing (E4/E10), timers (E11/max-hold/TokenRefresh), mute as privilege revocation with live roster snapshot (S7.5) — count is `--no-default-features --features embedded-web,stub-media`; mediasoup worker builds natively (3m52s) |
| docs/ (whitepaper + architecture + modules/01-09 + reference) | — | v0.2 + plan series |

## Phase Status

| Phase | Status |
|-------|:----:|
| 0 Scaffolding/config adaptation | ✅ (docs system ported + English migration; whitepaper/architecture/research generated 2026-09-28) |
| 1 core crate (P0-1) | ✅ 2026-09-29 — workspace + pixi toolchain + talkservo-core domain slice (plan-1 executed) |
| 1+ tooling (plan-6 D19/D20 + B/B2 D21 + C3 routes) | ✅ 2026-10-09 — facade default dir, secrets single-home, front_port yaml-driven, 28 lib tests, live LAN closed loop |
| 2-3 server/sfu + web/SDK (plans 2,3) | ✅ 2026-09-29 — signaling + SFU host; T1-T4 SPA/SDK, e2e browser-gated |
| 4 acceptance (LAN slice, D17) | ✅ SIGNOFF — 11 pass / 1 deferred (#8 FEC needs a real impaired network); posture = Alpha |
| 5 admin minimal surface (D18) | ✅ 2026-10-08 — /admin/api + console, click-through e2e |
| 6-8 ship tooling + wave-1 + build graph | ✅ 2026-10-19/21 — oxmgr facade + YAML single-source (D19-D21); plan-7 S1-S7.5 LANDED; plan-8 PolyOrch graph COMPLETE (fresh-clone 4/4) |

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
| D23 | Alpha wave-1 = trust & control: room authority chain + audit JSONL + tiers + intervention kit (+ UI/plan annexes; S7.5 walked A1/A2/A3) | ✅ adopted — S1-S7.5 LANDED 2026-10-20 |
| D24 | doc-audit 2026-10-19: four-lane findings adjudicated; forward pointers over wholesale rewrite | ✅ adopted |
| D25 | PolyOrch build assembly: submodule+CMake graph, scripts thin, runtime red lines | ✅ adopted |
| D26 | Verification topology: no hosted CI (Gitee-only remote); `scripts/gate.sh` in `.githooks/pre-push`, pixi lanes are push obligations, `ci.yml` kept as an inert job spec | ✅ adopted |
| D27 | Lane target vocabulary: one handle per (package, config), `talkservo-<object>-<verb>`, `stub|live` reused across knob/script/labels, consumers derived from the lane registry (plan-10) | ✅ adopted — landed 2026-10-22 |

## Next Steps

*Rewritten 2026-10-21: the previous list still advertised plan-1 items (workspace/pixi skeleton,
Floor state machine, "P0-1") as open work three revisions after they shipped, under a heading
AGENTS.md points readers at for "next implementation step". Stale instructions are worse than
none. Wave-1 is CLOSED (plan-7 S1-S7.5 + the three cross-client admin e2e rows + the last
deferred UI case, all by 2026-10-21). What is actually left:*

1. **Push the local queue** — as of this writing the wave-1 close is local-only (`git status -sb`
   carries the live count). Unpushed commits are the one remaining way this work can be lost.
2. **Supply-chain gate is live (closed 2026-10-21).** The earlier claim here — "`cargo-deny` is
   not installed" — was WRONG and is retracted: the binary sat in `$CARGO_HOME/bin` all along,
   invisible to the probe's PATH, so `gate-deny` reported Skipped forever (PIT-30). Fixed in
   `cmake/TalkServoGates.cmake` + the `audit` task, and the first real run immediately found a
   live advisory (RUSTSEC-2024-0449 `unsafe-libyaml` via deprecated `serde_yaml`) which was
   REMOVED rather than ignored, by migrating to `serde-saphyr`.
   Check: `ctest --test-dir out/cmake-build -L gate` shows gate-deny Passed (≈1.4s).
3. **Wave-2 CLOSED** (plan-9; W4's docs/tests landed with each slice): **W1 + W2 + W3 DONE 2026-10-21** — `/admin/ws` with its own
   `AdminEvent` contract (hints carry no row content, so `seq` stays a ledger-side concern), bounded hub
   with a counted eviction, its own 120/min connect budget, `docs/reference/admin-events.md`, TS mirror
   plus a Rust↔TS variant-set drift test, and the row probe that closed PIT-26; **W2 LANDED the
   same day** (console push-first, 30s reconcile / 3s degraded fallback, visible link state,
   debounced incremental Logs), which measured the push figure architecture §4 row 15 now
   carries. Landing W1 surfaced plan-9 R7 (below), and R7's own fix is what made the console
   adoption safe to ship.
   Observed while landing it: `gate-deny` failed once on a transient advisory-DB fetch and Passed on
   re-run — the now-live gate needs network, so a red gate-deny means check connectivity before
   believing a vulnerability. The plan's own §1/§6 claim "audit.rs is NOT touched" was corrected the
   same day: the file gained the hub sink; only `seq` is untouched.
   Old wording: proposed, not approved:
   (adjudicated, awaiting go) covers the D23 deferred family. Its §2 corrects the assumption carried here until now:
   an admin token cannot structurally pass `/ws` (AdminClaims has no `room`, auth.rs:96-104), so the
   push channel goes on a separate `/admin/ws` with its own `AdminEvent` contract — **plan-7 §4's
   "no new SignalingMessage variants" ruling does NOT need re-adjudicating**. Listen is the opposite:
   it is blocked by D13 producer-pause / acceptance #12, not by a missing wire variant, and the plan
   recommends deferring it. The R1-R5 walkthrough came back 2026-10-21 (R2 push+30s reconcile / R3 hint-only Logs, which dissolved R6 / R4 Listen deferred with its four prerequisites / R5 one socket per tab on its own connect budget): the plan is ADJUDICATED and awaits the go, no code yet.
4. **Alpha ops backlog opened by D24**: GAP-3 secret-rotation procedures, GAP-4 `audit.jsonl`
   ops story, GAP-5 Alpha ops doc home, GAP-6 Alpha gate consolidation.
5. **Trigger-gated — do NOT start early**: Electron shell (D10, Alpha), SDK npm publish
   (D14, Beta). (Codegraph indexing left this list: built 2026-10-21, see AGENTS.md.)
6. **R7 RULED (=B) AND LANDED 2026-10-21 — a kick is now terminal for the client.** The row probe
   proved the mechanism (`peer_joined` → `peer_left{kind:"removed"}` → `peer_joined` again, with a
   real second Join in the server log): `signal.ts` retried every remote close and only
   `expired_token`/`invalid_token` were terminal. Operator removals now travel as
   `RoomCommand::Remove{peer, reason}` and close with **4410**, which the SDK treats like an
   expired token — no backoff — while surfacing the operator's words; `Left` keeps `1000` so R1
   auto-reconnect still covers genuine drops. `pixi run e2e` has been green on that row since
   (2/2 under ctest, the previous reproduction path).
7. **Verification topology RULED =C and LANDED 2026-10-22 (D26).** No hosted CI exists: the only remote is
   Gitee, so the five GitHub jobs had never run. `ci.yml` stays as a portable job spec under an INERT banner;
   enforcement moved local — `.githooks/pre-push` runs `scripts/gate.sh` on every push (enable once per clone:
   `git config core.hooksPath .githooks`), `TS_GATE_FULL=1` adds `pixi run gate`, `--no-verify` is the
   deliberate escape. Still uncovered by anything, stated plainly in architecture §4: clean-machine mediasoup
   builds, the macOS `cargo check` target, and a cold cargo-deny — one host is the whole verification story,
   which is exactly why the silently-rotting ledger gates are the ones inside the hook.
8. **3c docs-drift pass LANDED 2026-10-22**: `docs/README.md` plan statuses corrected (plan-2 and plan-4 were
   still labelled `skeleton` after shipping; wave-1 read `DRAFT (review pending)` and wave-2 `awaiting go`, both
   closed 2026-10-21; the plan-5 row named `/a console`, a route C3 banned on 2026-10-09), the living-reference
   table gained the `admin-events.md` and fixture rows it never listed, architecture §4's wave-1 note now covers
   both waves, and the `signaling-schema.md` trigger was re-anchored (its condition had long been met while the
   doc stayed unwritten). The real remaining item behind that row — a declared-variant-set drift test for
   `SignalingMessage`, ~30 lines reusing the AdminEvent pattern — is recorded and UNWRITTEN: no external
   integrator needs a schema yet.
9. **lesson-review batch 2026-10-22 (C5 + PIT-34/35)**: the CI-location sweep turned into a new
   convention — **C5**, living docs may not present hosted/automatic verification as if it runs; name
   the command or mark it `inert`/`D26`. It is gate-embedded (5th block in `scripts/gate.sh`), and it
   caught five live offenders on first run (AGENTS Docker note, modules/09 three lines + one example
   label), which is the evidence that the rule earns its place rather than being written to file.
   PIT-34 records the near-miss where a blocking test reported success because the hook was untracked
   in a linked worktree — green from a mechanism that never ran; PIT-35 records an ad-hoc table checker
   reporting 8 phantom defects because it carried column widths across tables, and points back at the
   shipped doc gate instead.
10. **plan-10 LANDED 2026-10-22 (D27)** — the server family stopped being three names for two things
   (`talkservo-server` lib + `-bin` + `-mediasoup` -> `talkservo-server-stub` / `-live`), every button moved
   onto `talkservo-<object>-<verb>` with `clean` included, and the four hand-written name lists (web ordering
   edges, install handle, `talkservo.sh --lane`, `pixi run build`) are now derived from a lane registry. A
   stub-scope aggregate carries `ALL` because `build all` used to depend on nothing but the embedded PolyOrch
   directory — a bare `cmake --build` built nothing and exited 0. The mediasoup test verb is in-graph behind
   `TS_LANE_LIVE=1` (measured: Passed 1545s); landing it exposed that the ctest bridge lacked the linker pin,
   so the live lane had never been test-verified and `pixi run test-sfu` could not have passed. PIT-36/PIT-37/PIT-38/PIT-39
   record the traps found on the way (label substring widening a filter; a gate inheriting its
   neighbour's stale command variable). T5 then finished: cold `pixi run gate` 4/4 (gate-clippy 294.44s
   cold, 1.13s warm) and the fresh-clone drill green end-to-end (submodule at the recorded pin, 17 prefixed
   buttons, 12 ctest entries, `pixi run test` 5/5 BEFORE any build step, install staging `bin` + `dist`) —
   and the drill earned its keep by finding four things the warm tree had been hiding: the missing
   web/dist fixture (PIT-38), the undocumented npm provisioning (PIT-38, new `client-install` task +
   configure-time warning), a 1000ms Testing-Library budget racing a ~900ms jsdom mount (PIT-24
   recurrence), and a poisoned meson WrapDB cache that only `cargo clean -p mediasoup-sys` reveals
F4 then RULED =P1: the gitlink moved to 879856a (one commit past 5945f2a, upstream pnpm-workspaces
   face, still version 0.3.0) and is committed here — every session battery ran on 879856a while the
   fresh-clone drill ran the whole chain on 5945f2a, so both pins are verified and the recorded state
   now matches reality. The wrapper no longer hardcodes a SHA in its FATAL text. Pushing the local
   queue remains the top risk.
