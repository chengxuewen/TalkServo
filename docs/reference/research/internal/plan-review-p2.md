# Plan Review P2 — plan-2 server-sfu skeleton (peer review)

> 2026-09-28 | Read-only audit of [../../../plans/2026-09-28-plan2-server-sfu.md](../../../plans/2026-09-28-plan2-server-sfu.md) vs modules/02 (J/P/X/R/W/F), 03 (Sfu trait/features), 05 (E1-E11), 06 (security/config), D12/D13/D16, plan-1 Interfaces blocks. Bar = skeleton (task decomposition + interfaces + test names; not step-level). FROZEN — new findings go to a new dated file.

## Findings

| # | Sev | Finding | Source | Fix (one-liner) |
|---|-----|---------|--------|-----------------|
| 1 | HIGH | Observability absent: no task owns `tracing` bootstrap, closed event vocabulary, `room/peer/gen` fields, or log-derived `grant_latency_ms`/`media_recovery_ms` — acceptance #7/#10 read these from logs | modules/05 §Obs | Add to Task 6 (or new task): tracing JSON init + event-enum shared with core messages |
| 2 | MEDIUM | `ClientMediaBackend` trait invented in Task 1 — modules/03 dispatches via compile-time features (`sfu-mediasoup`\|`stub-media`), and D15 is client-crate design; a runtime trait here is a speculative abstraction with a confusing name | modules/03 §1 | Drop the trait; feature-gated `Sfu` impls only |
| 3 | MEDIUM | E11 detection seam undefined: `Sfu` trait has no media-activity/RTP-silence input, yet the watchdog consumes one and stub-media must fake it | modules/05 E11 | Name the seam during expansion (e.g. `AudioLevelObserver`-based signal; stub emits synthetic activity) |
| 4 | MEDIUM | Test-feature selection unstated: default = sfu-mediasoup, so which combo do `pixi run test` vs `test-sfu` run? Task 2/3 tests are stub-only but the gating mechanism is not pinned | plan Task 6 | Pin: stub suite = `--no-default-features --features stub-media`; live suite behind `test-sfu` (Linux) |
| 5 | MEDIUM | `TokenRefresh` proactive push missing (normative in modules/02 §1, D12); plan-1 explicitly deferred "push timing" to the server plan and it never landed | modules/02, plan-1 recap | Add to Task 4 timers: push at TTL − margin; test with fake clock |
| 6 | MEDIUM | E4 (transport-timeout retry→kick) and E10 (TRANSPORT_GUARDRAIL=50 + `MediaCapacity`) have test promises ("one per E row") but no implementation home in Tasks 2-4 | modules/05 E4/E10 | Assign E4→Task 3, E10→Task 3 transport-create path |
| 7 | MEDIUM | `scripts/issue-token.sh` + pixi `issue-token` task missing — modules/06 §PoC token issuance names them; dev/e2e Join depends on minted tokens (plan-1 doesn't create it either) | modules/06 §2 | Add to Task 5 deploy files or Task 6 gates |
| 8 | LOW | `Config` struct unowned: keys span two crates (server timers vs sfu RTC ports/TURN TTL); "config keys per modules/06" is a value list, not a placement | modules/06 §2 | Expansion: one env-backed struct per crate, fail-fast validation |
| 9 | LOW | Doc drift: modules/04 pins `docker/coturn.yml` "at P0-1" but plan-1 creates no docker/ files; plan-2 Task 5 is the de-facto home — no supersede annotation | modules/04 §1 | One-line annotation on modules/04 (live doc) when Task 5 lands |
| 10 | LOW | Cosmetic: duplicated `**Revision:**` v1.1 line (L9-11) | plan L9-11 | Delete one during expansion |

## Confirmed coverage (checks passed)

- **Decomposition**: J(T2+T3), P(T2+T3), X(via apply, T2), R(ServerSnapshot+gen-discard, T2), W(T1 supervisor+T3 MediaRestart), F(T4 E11+grace) all mapped; cooldown covered (constraints+T4); all 7 D16 routed items present incl. queue purge, deltas, `v`/caps, peer_left cascade, JWT `{sub,room,role}` + field-role rejection test (S-6/#14). MuteSet/ModeChange authority enforcement is test-routed; name its home in Task 2 during expansion (minor).
- **Interface chain**: consumes exactly plan-1's `apply()`/`SignalingMessage`/`FloorState` + modules/03 `Sfu`; Welcome/FloorQueued/ServerSnapshot shapes match. Sole invention = #2.
- **Task order**: timers (T4) after media orchestration (T3) is sane (E11 needs the media layer; E1 promote needs the apply loop). Linux-only gating stated (T1, T5). Live worker test covers E6/E11 kill paths.
- **Stub sufficiency**: no-op `Sfu` (canned TransportInfo/ProducerId, no-op apply_floor) is enough for fake-WS J/P/X + gen discard + collision reject as specced; gap limited to E11 seam (#3). Stub-level W message-flow test unstated but live W is in T5 — acceptable.
- **Deploy files**: coturn flags match modules/04 pinning (`use-auth-secret`/`static-auth-secret` env/`realm`); UDP 40000-40100; healthcheck on `/healthz` (exists in constraints + T5 + plan-1 stub). Only the P0-1 drift note (#9).

## Verdict

**plan-2 skeleton: NEEDS: observability task + event vocabulary (#1); resolve ClientMediaBackend to feature gates (#2); name E11 media-activity seam (#3); pin stub/mediasoup test-feature selection (#4); add TokenRefresh push (#5), E4/E10 homes (#6), issue-token script (#7). Decomposition, interface chain, ordering, stub strategy, and deploy pinning are otherwise sufficient for step-level expansion.**
