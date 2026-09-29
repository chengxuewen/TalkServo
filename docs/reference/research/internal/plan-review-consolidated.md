# Plan-Series Review — Consolidated (Lead Merge)

> 2026-09-28 | 4 lanes (p1 step-level / p2 skeleton / p3 skeleton / cross-chain). Raw 30 → 24 unique. Cross-checked with round-1/2 ledgers: F1 family (phantom schema step) independently caught by 2 lanes.

## Verdicts

| Plan | Verdict | Blocking count |
|------|---------|:---:|
| plan-1 (P0-1, step-level) | **NEEDS** — fix 4 blockers before execution | 4 |
| plan-2 (server+sfu) | **NEEDS** — 7 items before step-expansion | 1 HIGH |
| plan-3 (web+SDK) | **SUFFICIENT** — 5 expansion-stage items | 0 |
| series (cross) | **coherent** — 2 MEDIUM chain gaps | 0 CRITICAL |

## 🔴 BLOCKING plan-1 execution (P1 lane)

| # | Finding | Fix |
|---|---------|-----|
| P1-1 | Revision claims "+schema step" but no step materializes it (phantom, grep-verified, 2 lanes) | schemars as **dev-dep** (regular dep violates D5 core limit); integration test writes `docs/reference/signaling-schema.json` |
| P1-2 | `--features stub-media` on virtual workspace root fails; modules/09 already prescribes `talkservo-server/stub-media` | Task 2 Step 2/3 use `-p talkservo-server` form |
| P1-3 | D16 wire deltas (PeerJoined/PeerLeft{gen}, gen on PeerList, `v` on Join/Welcome) missing from Task 3 enum (cross F1) | extend Task 3 types + roundtrip fixtures |
| P1-4 | D16 queue-purge-on-Leave unassigned/untested = the genuine two-implementations ambiguity | Task 4: purge rule + position-rebroadcast test |
| P1-M | FloorEvent actor fields dropped vs modules/01 normative sig; wire casing (snake_case vs camelCase shorthand) unpinned for TS; ServerSnapshot↔role variants linkage; SystemTime+address-entropy contradiction (→ seeded deterministic constructor + SystemTime default); MAX_PEERS/MAX_QUEUE enforcement unassigned | fold into Task 3/4 during fix pass |

## 🟠 plan-2 (fix at expansion, 7): #1 observability bootstrap + log-derived metrics task (HIGH) · #2 drop invented ClientMediaBackend trait (server = compile-time features only; D15 is client-crate) · #3 E11 detection seam (AudioLevelObserver; stub emits synthetic) · #4 pin test-feature combo (stub suite `--no-default-features --features talkservo-server/stub-media`; live behind test-sfu) · #5 TokenRefresh push timing task · #6 E4→T3, E10→T3 homes · #7 issue-token.sh + pixi task owner

## 🟡 plan-3 (SUFFICIENT, expansion notes 5): routed findings lack per-task anchors (S-3 named T3/T4) · TS-gen+drift CI unowned (pairs with P1-1) · a11y toggle dropped from T4 · kill-switch endpoint + embed-feature naming · dup Revision lines + vitest coverage threshold

## 🔵 cross-chain (2 MEDIUM + 3 LOW): F1 = P1-3 family · F2 kill-switch endpoint orphan (= P3-4) · F3 plan-2 must declare fixtures deliverable · F4 dup Revision v1.1 lines (patch artifact, all 4 plans) · F5 plans 3/4 don't restate global constraints. INFO: rust-embed sequencing sound.

## Acceptance matrix (cross lane): §4 items 1-12 ALL owned; whitepaper §9 1-5 = §4 1-5 via plan-2 integration + plan-3 T5 + plan-4 sign-off. Registry matches files.

## Fix routing
- **Now (blocks plan-1)**: P1-1..P1-4 + P1-M batch → revise plan-1 in place (lead, one pass)
- **At plan-2 expansion**: 7 items → already appendable to plan-2 routed section
- **At plan-3 expansion**: 5 items → append to routed section
- **Editorial (now, trivial)**: F4 dup lines cleanup; F5 one-line constraint inheritance; F3 fixtures deliverable line in plan-2 T6
