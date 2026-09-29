# Plan-3 Skeleton Review — packages/client + Web SPA (p3-reviewer)

**Date:** 2026-09-28 | **FROZEN** — evidence snapshot; supersession annotated on live docs only.
**Scope:** docs/plans/2026-09-28-plan3-web-client-sdk.md (41 lines, v1.1 skeleton) vs docs/modules/08-web-ui.md, docs/modules/02-signaling-protocol.md, D12/D13/D14/D16, arch-review2-sdk S-items, plan-review-cross F-items.

## Skeleton-bar check (7 questions)

| # | Question | Answer |
|---|----------|--------|
| 1 | Four SDK layers → tasks mapping | ✓ complete: types + signal transport + wireStore (core state) → T1; media manager → T2 (modules/07 facade split honored). Routed items: all 8 gathered in the round-2 § ("land during expansion") but only micTruth/D13 (T2 body) and gen-discard (T1 body) anchor into task text; AudioHandle / token-race / connectionstate / visibilitychange have inferable owners (T1/T2, untagged); React owner rule (S-3) has NO task-body anchor in T3/T4 although S-3 explicitly named "plan3 Task 3/4". |
| 2 | Wire types source; drift CI owner | Hand-mirror stated (T1, consistent with modules/07). Generation task: UNOWNED. Drift CI: named in T1 only as "vs core JSON-schema once available" — not pinned to T6 gates; S-7 asked for "regen+diff CI gate in plan3 Task 1/6". Dependency risk: plan-1 L11/L13 revision line claims "+schema artifact step (review S-7)" but the plan-1 body carries no such step (grep = revision lines + docs/README planned-table only) — the artifact plan-3's drift check waits for is dangling. |
| 3 | SPA tasks vs modules/08 | Dispatcher ✓: grid + tally rings + event log (gen shown here only) + per-line mute/gain + FloorQueued queue strip + authorized actions (ModeChange/MuteSet/preempt; modules/08 says "mute-all" — MuteSet loop, acceptable at skeleton). Field ✓: roster, HTT tri-state, Space double-fire guard, keybind drawer, release-delay, aria-live, audio cues (grant/deny/taken), iOS unlock. DROPPED: the accessibility toggle of the settings drawer (modules/08 §1 field row) — absent from T4 and from Global constraints; "settings drawer" exists only as scattered keybind/release-delay items. |
| 4 | Playwright e2e — stub or real; plan-2 dep | REAL plan-2 server (banner prereq "plan-2 server APIs live" + Goal "against plan-2 server"); stubs correctly confined to vitest unit layer (mock WS T1, mock mediasoup handler T2). plan-2 dependency stated twice ✓. Residual: T5 "E11 via server-side kill switch endpoint" still consumes plan-review-cross F2's interface that no plan-2 task produces — F2 unresolved at skeleton level. |
| 5 | rust-embed boundary | T6 owns the embed, "feature-checked" ✓ — but the feature NAME is unpinned (modules/03 is the features source of truth) and F6's "note it as a server-crate amendment in plan-3 Task 6" was not applied. |
| 6 | npm posture (D14) | ✓ stated: Goal line "`packages/client` (unpublished)"; Beta npm timing owned by modules/07/D14 — no restatement needed. |
| 7 | Electron deliberately absent | ✓ deferral stated in banner ("Electron later consumes the same package"); modules/08 §Desktop-shell owns it at Alpha (D10). Implicit but unambiguous out-of-scope. |

## Findings

| ID | Severity | Title | Evidence | Recommendation |
|----|----------|-------|----------|----------------|
| P3-1 | MEDIUM | Routed round-2 findings lack per-task ownership | plan-3 L32-41: 8 findings under one "land during expansion" header; S-3 owner rule absent from T3/T4 bodies despite S-3 naming them | At expansion: tag each routed line with its owning task (T1/T2/T3/T4); write the S-3 rule (useSyncExternalStore + module-scope client + off() cleanup) into T3 and T4 |
| P3-2 | MEDIUM | TS generation + drift gate unowned; plan-1 schema step phantom | T1 "once available"; T6 gate list omits the drift check; plan-1 body lacks the step its own revision line claims | Assign regen+diff gate to T6 at expansion; chase plan-1 to actually land the schemars→schema artifact step (same family as cross-review F1) |
| P3-3 | LOW | Accessibility toggle dropped from field settings drawer | modules/08 §1 field row: keybind + release-delay + accessibility toggle; plan-3 T4 carries the first two only | Add the a11y toggle to T4 (one word) |
| P3-4 | LOW | F2 kill-switch still unproduced; F6 traceability note missing | T5 consumes kill-switch endpoint with no producing task; T6 lacks the amendment note + named feature | Expansion: confirm plan-2 T5 produces the endpoint; name the embed feature; append F6 note to T6 |
| P3-5 | INFO | Known cross-review items recur here | F4: duplicate Revision lines (L9/L11); F5: no vitest coverage threshold in Global constraints | Fold both into the shared expansion-cleanup pass across all four plans |

## Verdict

**plan-3 skeleton: SUFFICIENT** — all 7 bar questions answerable; architecture, dependency chain, real-server e2e split, publishing posture, and Electron deferral are correct. NEEDS at expansion (not skeleton-blocking): P3-1 pin every routed finding to a task (S-3 into T3/T4), P3-2 own the drift gate in T6 and resolve plan-1's phantom schema-artifact step, P3-3 restore the a11y toggle, P3-4 close F2/F6.
