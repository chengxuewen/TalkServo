# Plan-Series Cross-Review (plan-1 → plan-4 as a chain)

**Date:** 2026-09-28 | **FROZEN** — evidence snapshot; supersession annotated on live docs only.
**Scope:** docs/plans/{2026-09-28-p0-workspace-core, plan2-server-sfu, plan3-web-client-sdk, plan4-deployment-acceptance}.md + docs/README.md registry + architecture.md §4.

## Findings

| ID | Severity | Title | Evidence | Recommendation |
|----|----------|-------|----------|----------------|
| F1 | MEDIUM | D16 wire additions routed past their owning plan | plan-2 round-2 §adds `v` on Join, `PeerJoined`/`PeerLeft`, `Error{BadVersion}`; plan-3 consumes "wire v:1" — but these are `talkservo-core` `SignalingMessage` types, produced by plan-1 Task 3. plan-1 has no round-2 section carrying them. | Add the D16 wire deltas to plan-1 (core crate owner) during expansion, or state explicitly that plan-2 Task 2 amends core types first. |
| F2 | MEDIUM | plan-3 consumes an interface no plan produces | plan-3 Task 5: "E11 via server-side kill switch endpoint" — plan-2 defines the E11 watchdog (Task 4) but no test kill-switch endpoint in any task or produce list. | Pin the kill-switch endpoint in plan-2 Task 5 produces (e.g. dev-only POST /__test/kill-uplink, feature-gated). |
| F3 | LOW | "plan-2 fixtures" never declared | plan-3 Task 1: "replay fixtures from plan-2 fixtures"; plan-2 Task 6 produces only "ServerApi surface list". | Add "J/P/X/R replay fixtures (JSON)" to plan-2 Task 6 produces. |
| F4 | LOW | Revision lines duplicated in all 4 plans | plan-1 L11+L13 (divergent tails), plan-2 L9+L11 (identical), plan-3 L9+L11, plan-4 L7+L11 — patch-script artifact pattern. | Dedup to one Revision line per plan at next touch. |
| F5 | LOW | Constraint headers not uniform | plan-2 inherits plan-1 by reference ("all plan-1 constraints"); plan-3/4 restate only UI/deploy constraints — lock-commit discipline and clippy/coverage scope (incl. TS coverage) unstated there. | One line in plan-3/4: "inherits plan-1 global constraints where applicable"; add vitest coverage threshold in plan-3. |
| F6 | INFO | rust-embed edits plan-2's crate from plan-3 | plan-3 Task 6 embeds `web/dist` into `talkservo-server`. Sequencing is sound (after plan-2) but it is a server-crate amendment. | Note it as such in plan-3 Task 6 so server-scope changes stay traceable. |

## Interface chain (consumed → defined earlier?)

- plan-2 consumes `core::apply`, wire enums, pixi tasks, feature names — all defined plan-1 ✓ (except F1's D16 deltas).
- plan-3 consumes wire v:1 (F1), plan-2 server APIs (Task 6 records ServerApi ✓), fixtures (F3), kill-switch (F2).
- plan-4 consumes server+client (prereq stated ✓), `floor_media_grace_ms` (plan-2 Task 4 ✓), room-TTL reap logs (plan-2 round-2 ✓), docker/coturn (plan-2 Task 5 ✓).

## Acceptance coverage matrix (architecture §4 1–12 × plans)

| §4 item | plan-1 | plan-2 | plan-3 | plan-4 |
|---------|--------|--------|--------|--------|
| 1 A holds → B/C denied | rules 1-9 tests | T2/T4 integration, E-rows | T5 e2e | sign-off |
| 2 A audible to B/C | — | T5 grant→RTP flows | T5 getStats | T2 P-seq |
| 3 release → Idle | T4 | T2 | T5 | sign-off |
| 4 strictly-higher preempt | T4 | T2/T4 | T5 | sign-off |
| 5 holder drop → auto-release | T4 (MediaDown) | T4 E-rows | — | T4 |
| 6 R1 snapshot-resync | — | T2 (ServerSnapshot/R) | T5 reconnect R1 | sign-off |
| 7 E6 worker kill | — | T5 (E6) | — | T4 (real timing) |
| 8 FEC A/B ≥30% | Opus pins | — | — | T3 |
| 9 ICE host/srflx/relay | — | T5 (coturn) | — | T2 |
| 10 P-latency 300/600ms | — | — | — | T2 |
| 11 zombie holder E11 | MediaDown semantics | T4/T5 | — | T4 (+D16 dual-gate) |
| 12 D13 gating zero payload | — | T3 pause-unless-granted | T2 publish-pause UI | T4 tcpdump bytes |

No unowned items. Whitepaper §9 five criteria = §4 items 1–5, covered by plan-2 (integration) + plan-3 T5 (e2e), signed off in plan-4.

## Sequencing

Prereqs stated in all skeleton banners: plan-2←plan-1, plan-3←plan-2, plan-4←plans 2-3 ✓. plan-3 T5 e2e needs plan-2 server live — stated ("against plan-2 server"). Parallel feasibility: plan-3 Tasks 1–2 (SDK vs mock WS) can run concurrently with plan-2; Tasks 3–6 are serial after plan-2 Task 2+. No write-conflicts on files (plan-3 T6's server edit is the only cross-crate touch, F6).

## Registry & proportion

docs/README plans table: 4 rows, filenames and statuses match files exactly ✓. Proportion: plan-1 expanded (149 lines, as designed); skeletons 28/40/41 lines — plan-4 thinnest but tasks+prereqs+review-focus pinned; adequate for skeleton tier.

## Verdict

**series: coherent** — gaps F1–F3 are interface-routing fixes resolvable during skeleton expansion; no architectural contradiction, no unowned acceptance item.
