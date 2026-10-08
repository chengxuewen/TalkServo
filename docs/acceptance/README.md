# Acceptance artifacts (plan-4)

Committed quantitative evidence for architecture.md §4. Regenerate e2e rows:
`cd web && npx playwright test` (signaling) / `TS_E2E_FEATURES=live npx
playwright test` (media+latency, live worker).

| File | Content | Status |
|------|---------|--------|
| e2e-results.json | 12-row acceptance registry (per-row status + evidence) | regenerated per run |
| latency.json | keydown→audible p50/p95 (client-relative) | **measured: p50=89ms** (LAN budget 300) — harness `TS_E2E_FEATURES=live npx playwright test --grep "latency harness"` |
| fec-ab.json | concealment-share A(off)/B(on) under 12% netem | pending — TS_NETEM=1 + scripts/netem.sh |
| SIGNOFF.md | final 12-item table with evidence links | VPS slice |

## Current status (2026-09-30, local slice)

12 items per architecture.md §4: **10 pass / 1 partial / 1 pending**.
Partial-adjudicated: #8 (FEC A/B measured 22.7% under the 12%-loss loopback
two-hop cascade; per ruling the >=30% budget line is decided at VPS T5 with a
single-leg WAN impairment — loopback distorts it by construction; raw arms in
fec-ab.json). Pending: #9 (ICE classes host/srflx/relay — public VPS).
Notable measurements: keydown→audible p50 = **89 ms** (LAN budget 300);
audible verified by inbound-RTP byte growth on the listener recv transport.

Caveats: loopback rows marked `local-loopback` (WS collateral acceptable —
TCP retransmits; veth upgrade path for clean isolation at sign-off). E11
watchdog × fake-device beep gaps down the speaker ~2s into every hold — the
audible/latency probes sample in the early hold window; real speech does not
trigger the watchdog. Headless WebAudio measures 0 RMS on remote streams —
audibility truth is inbound RTP byte growth, not analyser RMS.
