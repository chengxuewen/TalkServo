# Plan Review p1 — plan-1 (P0 Workspace + talkservo-core) Step-Level Review

**Date:** 2026-09-28 | **FROZEN** — evidence snapshot; supersession annotated on live docs only.
**Scope:** docs/plans/2026-09-28-p0-workspace-core.md (v1.1) vs modules/01,02,03,09 + D1-D16. Bar: writing-plans self-review (spec coverage, step ambiguity, type consistency, proportion).
**Read with:** [plan-review-cross.md](plan-review-cross.md) (series-level; its unresolved F1 re-filed here as P1-3).

## Findings

| ID | Severity | Title | Evidence (plan file unless noted) | Recommendation |
|----|----------|-------|-----------------------------------|----------------|
| P1-1 | HIGH | Schema-artifact step claimed but absent | L11+L13 claim "+schema artifact step (review S-7)"; no task/step in Tasks 1-5 creates it | Materialize the step in Task 5. Feasibility: schemars as **dev-dependency** (`schemars = "1"`, Draft 2020-12, honors serde tag/rename_all) + integration test `tests/schema.rs` emitting `docs/reference/signaling-schema.json`. A `[dependencies]` schemars would violate the core dep limit (L20 / D5: serde/serde_json/thiserror only); dev-deps do not ship to consumers. Sync filename with docs/README planned-table (`signaling-schema.md` today — pick .md or .json in both places). |
| P1-2 | HIGH | Virtual-workspace feature syntax regression (H-3) | L72+L79 `cargo check --workspace --no-default-features --features stub-media` against a `[workspace]`-only root (L61); bare `--features` fails in a virtual workspace with sibling members both defining `stub-media` (L63, L71) | Use the modules/09 §3 (L47) prescribed form: `--features talkservo-server/stub-media` (and/or `-p talkservo-sfu --features stub-media`) in Task 2 Step 2 and the CI mac leg. As written, Step 2 hard-fails = execution stall. |
| P1-3 | HIGH | D16 wire deltas missing from Task 3 enum | L95-96: no `PeerJoined{info,generation}`/`PeerLeft{peer,generation}`; `PeerList{peers}` lacks gen (D16(2)); `Join{jwt}`/`Welcome` lack `v` + echo (D16(3)) | Add to Task 3 types + roundtrip samples. Cross-review F1 routed exactly this to plan-1 and v1.1 did not absorb it. `Error{code}` covers BadVersion — pin the code constant. |
| P1-4 | HIGH | D16(1) queue purge unpinned | Task 4 (L113-L128): no apply() instruction nor test for Leave of a **queued** peer; D16(1) requires purge + position rebroadcast | Pin: `Leave{peer}` purges queue entry, emits resequenced `FloorQueued{position}` to remaining; add one test. Otherwise two reasonable implementations (purge vs ghost entry) — the exact ambiguity the step-scan bar rejects. |
| P1-5 | MEDIUM | FloorEvent drops actor vs normative modules/01 type | L113 `ModeChange{mode}`/`MuteSet{peer,on}` vs modules/01 L32 `ModeChange{mode,actor}` / `MuteSet{peer,on,actor}`; L123 prose "core trusts actor" | Add actor fields (apply may ignore) or amend modules/01 via user-approved design change. Silent divergence from a normative signature fails ledger discipline. |
| P1-6 | MEDIUM | Wire field names unpinned vs modules/02 shorthand | Plan snake_case (peer_id, turn_creds, rtp_parameters, producer_id, position, generation — L95-96) vs modules/02 L8-9 camelCase (peerId, turnCreds, rtpParameters, producerId, pos, gen); `rename_all` renames variants only, not fields | Annotate modules/02: table names conceptual, wire = snake_case; pin exact Rust field names in Task 3 — plan-3 hand-written TS unions and the P1-1 schema both consume them. |
| P1-7 | MEDIUM | Snapshot payload attachment ambiguous | `ServerSnapshot` is a unit variant (L95) while `FieldSnapshot`/`DispatchSnapshot` exist separately (L97) with no linkage | Pin one shape: `ServerSnapshot(Snapshot)` with `enum Snapshot { Field(FieldSnapshot), Dispatch(DispatchSnapshot) }` (role known from JWT at join). |
| P1-8 | MEDIUM | Entropy source self-contradictory / untestable | L112: "SystemTime nanos XOR address entropy — D5 dep limit: use SystemTime only" — names two implementations; address entropy is nondeterministic across ASLR configs | Split: `initial_seeded(mode, u64)` (deterministic; used by all rule tests) + `initial(mode)` seeded from `nanos ^ pid` (stdlib only). Rand-free: rules 1-9 tests are gen-agnostic (L31 pins fresh-state), so ≥80% coverage stays deterministic; keep the two-calls-differ boot-offset assert probabilistic-only. |
| P1-9 | MEDIUM | D16(4) cap enforcement unassigned | `MAX_PEERS/MAX_QUEUE` overflow → Denied (D16(4)); L149 boundary note lists server-domain items but not caps; Task 4 has no queue-full path | Assign explicitly: either core `apply()` (needs cap input) with a test, or server pre-check — say which in Task 4 or the L149 list. |
| P1-10 | LOW | Coverage gate not enforceable as written | L24/L127 require core ≥80%; `pixi run coverage` = `tarpaulin --workspace` (modules/09 L43), no fail-under; aggregate can mask core | Task 5 Step 1: document `cargo tarpaulin -p talkservo-core --fail-under 80` as the exit check. |
| P1-11 | LOW | "per modules/09 §3 sketch verbatim" is not | L40 task list omits `web-dev` and `ci-test-mediasoup` that modules/09 §3 (L44, L48) defines | Add both lines; plan-3 consumes `web-dev`, and the CI test-mediasoup job should call the pixi task. |
| P1-12 | LOW | Editorial residue in normative sections | Duplicate divergent Revision lines L11/L13 (cross-review F4); SmolStr deliberation inside the "exact" interface L91; think-aloud in L32 | Clean. Interfaces labeled "exact, consumed by Task 4/5" must not carry rejected alternatives or deliberation. |
| P1-13 | LOW | CI boundary acceptable but uncited | Task 2 (L72) owns ci.yml; modules/09 §1/§3 owns design (mac = check-only stub, L12) — consistent; platform.md "test on both OSes" row is stale pre-D11 body | Add one citation line in Task 2 ("implements modules/09 §1/§3") so future CI edits route through modules/09; treat platform.md body as conditional per its D11 banner. |

## Coverage notes (review questions 1, 5, 8)

- modules/01 rules 1-9 → Task 4: 1✓ 2✓(MuteSet) 3✓(both switch directions + Hybrid cap) 4✓ 5✓ 6✓(MediaDown immediate, grace=server, pinned) 7✓ 8✓(no-ops, dup-Request, gen stability) 9 = policy-only, correctly untested.
- **D15 client backend trait: correctly absent** — Beta scope per D7/D8; no plan-1 obligation, and none should be added.
- Proportion: 149 lines justified — normative test matrix is the payload, not padding; P1-12 trim is the only fat.
- Q8: coverage ≥80% is deterministic-achievable once P1-8's seeded constructor lands; the SystemTime path alone leaves one probabilistic assert.

## Verdict

**plan-1: NEEDS: P1-1..P1-4 blocking before execution (schema step materialization, workspace feature syntax, D16 wire deltas, queue-purge pinning); P1-5..P1-9 fold into the same one-pass edit; P1-10..P1-13 cleanup.**
