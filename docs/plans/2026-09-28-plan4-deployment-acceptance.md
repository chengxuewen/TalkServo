# Plan 4: Deployment + Acceptance — Step-Level Plan

> **Revision:** v2.0 (2026-09-30) — expanded at execution start. Splits into a
> **local-complete slice** (T1-T3, runs on this host today) and a
> **VPS slice** (T4-T6, blocked on public host provisioning).
>
> **Goal:** 12-item acceptance signed off with committed quantitative
> artifacts. Exit = whitepaper §9 + architecture §4 complete.

## Local-complete slice (this host, Linux)

### Task 1: deploy stack — local parity
- [ ] 1.1 `docker/Dockerfile` release-build verification (builds, healthz green in compose)
- [ ] 1.2 `docker/compose.yml` + `coturn.yml` — `docker compose up` smoke: server healthy, coturn listening 3478 (container interconnect only)
- [ ] 1.3 systemd unit template `deploy/talkservo.service` (Restart=always, env file) — installable, documented
- [ ] 1.4 Caddy config template `deploy/Caddyfile` (wss/https termination, self-signed + LE variants)
- [ ] 1.5 commit `feat(deploy): stack templates + compose smoke`

### Task 2: fake-media e2e (A2) — unlocks acceptance #1-#8 automation
- [ ] 2.1 Playwright launch args `--use-fake-device-for-media-stream --use-fake-ui-for-media-stream`; field page mic permission auto-grant
- [ ] 2.2 SDK media manager: real mediasoup-client wiring (device.load(routerCaps), produce with opusFec codecOptions, consume + `<audio>` attach) — replace recv-factory DI with the real stack behind a flag
- [ ] 2.3 e2e rows: holder audible (listener `<audio>` non-silent via AnalyserNode RMS), #6 R1 reconnect mid-hold state restore, #7 E6 worker kill → recovery time measured, #11 E11 (observer-driven MediaDown within grace — live host required)
- [ ] 2.4 #12 byte-count proof: listener-side getStats bytesReceived delta ≈ 0 for idle (non-granted) peers over a 5s window (RTCP-only)
- [ ] 2.5 artifacts: `docs/acceptance/e2e-results.json` (per-row pass + timings) committed
- [ ] 2.6 commit `feat(e2e): fake-device media matrix — acceptance #1-#8 automated`

### Task 3: latency + FEC harness — local LAN segment
- [ ] 3.1 client-relative latency harness: t0=keydown, t1=AnalyserNode RMS threshold on listener; p50/p95 over 20 presses (LAN box loopback)
- [ ] 3.2 FEC A/B: getStats concealment-share delta with netem 12% loss on loopback interface (documented dev-lo caveat; veth upgrade path noted)
- [ ] 3.3 artifacts: `docs/acceptance/latency.json` + `fec-ab.json` committed (rows marked `local-loopback`, budget verdicts pending public run)
- [ ] 3.4 commit `feat(acceptance): latency + fec harness with local artifacts`

## VPS slice (blocked on public host)

### Task 4: public deployment
- [ ] 4.1 provision VPS (ubuntu 22.04) → run deploy/ stack + Let's Encrypt
- [ ] 4.2 coturn E2E: relay ICE candidate completes a P-sequence (#9 relay row)
- [ ] 4.3 TURN 1h-boundary drill (review-focus pin)

### Task 5: full 12-item sign-off
- [ ] 5.1 #9 ICE matrix (host/srflx/relay) on public net
- [ ] 5.2 #10 P-latency on LAN + 4G budgets (revises architecture.md line with data)
- [ ] 5.3 #3 netem 12% ×3 runs (veth, no loopback caveat)
- [ ] 5.4 acceptance table completed in docs/acceptance/SIGNOFF.md with evidence links; deviations logged
- [ ] 5.5 ledger: status.md → PoC-complete; lesson-review

## Global constraints (inherit plan-2/3 + this plan)
- wss/https only on public; TURN TTL 1h; secrets env-only; every artifact JSON committed
- review-focus pins: clock-offset→relative deltas; netem on dedicated console; mobile-background degradation documented-not-fixed
