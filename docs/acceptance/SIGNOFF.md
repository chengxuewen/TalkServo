# PoC Acceptance Sign-off — architecture.md §4 (12 items)

**Range**: D17 — LAN same-subnet, local chromium simulation (fake-device), loopback + self-hosted coturn. Generated 2026-10-08. Repro commands per row in `docs/acceptance/README.md` + `e2e-results.json`.

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | PTT exclusive grant, others denied | **pass** | floor.spec (P/X/R rows) + media_flow denial tests |
| 2 | A audible to B/C (consume chain) | **pass** | live e2e `holder audible`: ProducerAvailable → ConsumeOk → audio element → **inbound-RTP byte growth** (audibility truth; headless WebAudio RMS is 0 — see fec-ab/latency methodology) |
| 3 | Release → idle | **pass** | floor.spec + floor_rules r2 |
| 4 | Preempt strictly-higher | **pass** | floor.spec X row + r5 |
| 5 | Holder drop → auto-release | **pass** | timers_flow E1 + live_e11 rows |
| 6 | R1 reconnect snapshot restore | **pass** | floor.spec R1 row |
| 7 | E6 worker kill recovery | **pass** | live_e6_kill_notifies_and_respawns (real worker, respawn clean) |
| 8 | FEC A/B ≥30% concealment reduction under loss | **partial — deferred** | 4-run chain in `fec-ab.json`: codec toggle PROVEN on the wire (useinbandfec 0/1); FEC recovery demonstrated (10.4% concealment with NACK disabled at 140ms constant delay) but loopback variance (playout-buffer adaptation, 4-10pp) cannot adjudicate ≥30%. **Deferred to a real impaired network** (device + degraded Wi-Fi or WAN tc) per user ruling 2026-09-30/10-08. PIT-8 documents the method limits. |
| 9 | ICE classes host/srflx/relay full P-sequence | **pass (LAN range)** | `ice-matrix.spec.ts` vs coturn on-LAN: host = every live test; srflx gathered via STUN (external-ip masquerade — without it Chrome dedups srflx≡host on a NAT-less LAN); relay: policy=relay both legs, TURN allocation + CHANNEL_BIND proven in coturn logs, **audio flowed** |
| 10 | keydown→audible ≤ budget | **pass — budget revised** | `latency.json`: p50 314-328ms (reused-consumer path, warm-up separated). Budget line revised with data per its own clause: **LAN 350 / public 600**. Methodology: 60ms poll operating point; pre-D13-reuse the path was 600-1500ms (PIT-10) |
| 11 | E11 zombie-holder: floor frees on media death | **pass** | timers_flow + live_e11_activity_truth_source (AudioLevelObserver truth); e2e E11 row = stub-synthetic |
| 12 | D13 gating: zero idle uplink payload | **pass** | live e2e idle byte-count row + apply_floor diff (stub call-log + live stats) |

**Tally: 11 pass / 1 partial(deferred) / 0 unmet.**

## Deferred & follow-ons (not PoC blockers)
1. #8 ≥30% formal adjudication — real impaired network (device + Wi-Fi degradation, or Alpha-stage VPS single-leg tc). Harness ready (`scripts/netem.sh media` + `--grep "#8"`).
2. Public-network posture (Let's Encrypt, WAN TURN, 4G candidate ladder) — Alpha scope, D17.
3. Real-device LAN smoke (phones/laptops over `https://<lan-ip>` with the distributed root cert) — recommended before field use; `scripts` + Caddyfile LAN block ready.

## Ledger quality notes (methodology captured during this slice)
- PIT-5/6/7: wire-id quoting, transport dir-keying, 0.0.0.0+announced bind death (all fixed + test-covered)
- PIT-8: loopback netem two-leg cascade limits (measurement methodology)
- PIT-9: env http_proxy captures chromium LAN traffic (direct:// fix)
- PIT-10: plan wording silently contradicting D13 — consumer reuse restored, budgets re-measured
