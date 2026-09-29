# Arch Review Round 2 — Remediation Verification (21 fixes)

> 2026-09-28 | arch-review2-team / remediation-verifier | READ-ONLY verification lane. FROZEN: this dossier records the state of the live docs at verification time; it is never back-written — new findings go to a new dated file. Source dossier: `architecture-review-consolidated.md` (round 1, 21 findings). Evidence = live-doc file:line.

## Findings

| # | Sev | Title | Evidence | Recommendation |
|---|-----|-------|----------|----------------|
| R2-1 | MEDIUM | Stale pre-D12 event shape in modules/04: `FloorGranted(holder)` prose survives while the wire table (modules/02) and FloorState define the grant-set payload `FloorGranted{grants,gen}` — the exact single-holder vocabulary finding #3 removed. A reader implementing from modules/04 would emit the old message. | modules/04-media-pipeline.md:15 "`FloorGranted(holder)` resumes the holder producer…" vs modules/02-signaling-protocol.md:9 "`FloorGranted{grants,gen}`"; modules/01-floor-model.md:24 `grants: Vec<PeerId>` | Reword 04:15 to `FloorGranted{grants}`: resume every granted producer + matching consumers. |
| R2-2 | LOW | modules/05 E10 still asserts "PoC cap ~50 transports" unqualified — dossier #16 ruled the ~50 figure an invented capacity claim; modules/06 now correctly carries `TRANSPORT_GUARDRAIL=50(arbitrary,measure)`. Two docs, one number, two epistemic statuses. | modules/05-error-model.md:20 vs modules/06-deployment-security.md:23 | E10: replace "PoC cap ~50 transports" with "config guardrail (modules/06), measure at acceptance". |
| R2-3 | LOW | "Holder" is never explicitly redefined after D12. Rule 4 implies holder ≡ `grants[0]` (preempt compares `grants[0].user_priority`), and event/UX prose keeps using holder (`Leave(holder)` diagram, `MediaFailed{holder}`, tally red=holder). Semantics stay coherent for Exclusive/Hybrid but Open mode has no holder while 05:E11 row is holder-phrased. | modules/01-floor-model.md:40 (`grants[0]`), :16, :42; modules/05-error-model.md:11; modules/08-web-ui.md:10 | One line in modules/01 after the struct: "holder ≡ grants[0] (Exclusive/Hybrid); Open mode speaks of grantees instead." |
| R2-4 | LOW | Master acceptance list ordering broken by the fix insertion: items 11, 12 sit between 7 and 8. Cosmetic, but the list is normative and renumbering will churn diffs. | docs/architecture.md:69-75 (6,7,11,12,8,9,10) | Reorder to 6-12 or renumber appended items as 8-12. |
| R2-5 | INFO | E11 interplay (watchdog 2 s vs consent 30 s) is coherent but precedence is ledger-only: modules/05 lists both as alternative detectors without stating "watchdog fires first, consent is the backstop"; D13 Limits records the reconciliation (paused producers keep RTCP/consent → watchdog stays valid) in `.agents/memorys/decisions.md` only. | modules/05-error-model.md:11; modules/02-signaling-protocol.md:29; decisions.md:123 | Optional: append "watchdog primary, consent backstop" to the 05:E11 Detection cell. |
| R2-6 | INFO | coturn pinning is deferred to `docker/coturn.yml` at P0-1 (per dossier #18 fix target); flags/realm are enumerated, nonce-lifetime choice (600 s vs TURN TTL 3600 s) is not recorded anywhere live yet. | modules/04-media-pipeline.md:10; modules/06-deployment-security.md:19 | When P0-1 lands the yml, verify nonce lifetime ≤ TURN_TTL_S and note it in 06. |

## Verified clean (fix present, no new contradiction)

1. **#1 zombie-holder** — MediaDown+grace path consistent across all four docs: 01:42, 02:29 (F-seq), 05:11 (E11), architecture.md:71 (acceptance #11). WS-drop grace=0 vs media grace=2000 kept distinct.
2. **#2 TLS/wss** — 06:15 Caddy/axum-rustls, self-signed PoC, wss outside LAN; unblocks acceptance #9.
3. **#3 grant-set FloorState (D12)** — grants/muted/queue struct 01:22-28; mode caps 1/2/N (MCPTT dual-floor) rules 1-3; no `Option<PeerId>` remnant anywhere in live docs (repo grep = 0); preemption vs `grants[0]`.
4. **#4 ModeChange/MuteSet** — wire table 02:8-9, FloorEvents 01:31-32, authority gate 06:21, dispatcher actions 08:10. Consistent quad.
5. **#5 FloorQueued** — 02:9, 01:37, queue strip 08:10.
6. **#6 apply_floor unified** — 02:18 (diff, resume not create), 03:28 (created-once-per-pair, pause/resume diff, crate-verified note), 03:35-40 (state-not-events trait). No create/close wording left on the grant path; 02:25 re-creation is post-worker-death (W-seq), correctly out of scope of create-once.
7. **#7/D13 gating** — 02:17, 04:15, acceptance #12 (architecture.md:72). D13 vs E11 non-contradiction confirmed: paused producers keep RTCP/consent (decisions.md:123), watchdog duty intact.
8. **#8 trust statement** — 06:16 + OQ-10 (architecture.md:60) Beta gate.
9. **#9 role-scoped snapshot** — 02:11 (queue dispatcher-only) + D12 citation; consistent with 07:46 role-scoped dispatch.
10. **#10 AlreadyJoined** — 02:17 + 02:31 error codes + OQ-11 Alpha displace.
11. **#11 TokenRefresh** — 02:9/02:11 push-before-expiry; complements (not contradicts) E9 hard-expiry rejoin 05:19; 07:46 silent refresh.
12. **#12 queue semantics** — 01:26 FIFO-within-tier/priority-across; transition directions rule 3; starvation ceiling rule 9.
13. **#13 DenyReason closed** — 01:47 incl. RateLimited/PreemptPriority; 02:19 uses it.
14. **#14 max-hold** — 45 s dispatch default 01:43 = 06:23 config key; within dossier's 30-60 s band.
15. **#15 TCP fallback** — candidate order UDP → TURN-TLS/443 → ICE-TCP 04:10; compose TCP range noted; ICE-Lite no-restart consequence present.
16. **#16 worker placement** — `Supervisor::acquire_worker()` round-robin behind interface 03:28 (crate verified); guardrail keyed in 06:23 (see R2-2 for the 05 residual).
17. **#17 /healthz** — 03:29 with compose/CI readiness purpose.
18. **#18 coturn** — pin delegated to P0-1 file (see R2-6).
19. **#19 Electron mechanics** — all three (single-instance lock, render-process-gone→reload+R-seq, Linux auto-update=Alpha) plus C-4 register() conflict UX in one paragraph, 08:30.
20. **#20 join-flood** — accepted-risk wording 06:22 with Alpha middleware pointer.
21. **#21 pixi.lock rule** — 09:71 mandatory alongside Cargo.lock; reinforced in plans/p0 (plans/2026-09-28-p0-workspace-core.md:15,51).

Cross-checks that passed: mono codec has no 2ch remnant (04:9 stereo = program-audio exception only); whitepaper holder/relay prose (142/174) is planning-voice, superseded via revision note architecture.md:91 per docs/AGENTS (whitepaper not restyled); D12/D13 resolve in `.agents/memorys/decisions.md`; no fix contradicts another (D13-pause × F-seq MediaDown × E11 watchdog triangle closes cleanly).
