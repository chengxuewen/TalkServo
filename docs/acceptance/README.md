# Acceptance artifacts (plan-4)

Committed quantitative evidence for architecture.md §4. Regenerate e2e rows:
`cd web && npx playwright test` (signaling) / `TS_E2E_FEATURES=live npx
playwright test` (media+latency, live worker).

| File | Content | Status |
|------|---------|--------|
| e2e-results.json | 12-row acceptance registry (per-row status + evidence) | regenerated per run |
| latency.json | keydown→audible p50/p95 (client-relative) | pending local run — harness in web/e2e/media.spec.ts |
| fec-ab.json | concealment-share A(off)/B(on) under 12% netem | pending — TS_NETEM=1 + scripts/netem.sh |
| SIGNOFF.md | final 12-item table with evidence links | VPS slice |

Caveats: loopback rows marked `local-loopback` (WS collateral acceptable —
TCP retransmits; veth upgrade path for clean isolation at sign-off).
