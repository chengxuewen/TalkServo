# Plan 4: Public Deployment + Acceptance — Task-Level Plan

> Skeleton plan: tasks pinned; step expansion at execution start. Prereq: plans 2-3 landed (server + client). This plan's exit = whitepaper §9 + architecture §4 acceptance (12 items) signed off.

**Goal:** Public-internet deployment (VPS + coturn + TLS) proving acceptance items #9-#12 and the full 12-item sign-off, with quantitative artifacts committed.

**Revision:** v1.1 (2026-09-28) — T4 chaos rows now verify D16 dual-gate (#12: client track.disabled AND server pause; zero payload bytes) + E11 zombie row; room TTL reap visible in logs.

**Architecture:** single VPS running talkservo-server (native Linux build) + coturn container + Caddy TLS termination; clients = browser LAN + 4G field + Electron desktop; weak-net via tc netem.

**Spec:** docs/modules/04 (harness), 06 (security), architecture §4 acceptance, modules/05 E-rows, research/internal consolidated + arch-review2-consolidated.

**Global constraints:** wss/https only; TURN creds TTL 1h signed; no secrets in repo (env only); every measurement artifact (JSON/CSV) committed under `docs/acceptance/`.

### Task 1: deploy stack
VPS provision: server binary (release build via CI artifact), coturn container (pinned flags), Caddy (wss+https, self-signed→Let's Encrypt), systemd units + restart policy (E7 partial: planned restart drops rooms — documented), /healthz checks.
### Task 2: ICE matrix + latency
Acceptance #9: host/srflx/relay each complete a P-sequence; latency harness (client-relative deltas; t0 keydown, t1 AnalyserNode RMS threshold) — LAN ≤300ms, relay ≤600ms lines; artifacts committed.
### Task 3: weak-net FEC A/B
netem on server egress (12% loss, 60s windows, 3 runs): concealment-share formula per modules/04, pass ≥30% reduction; on `dev lo` caveat documented (WS collateral acceptable, TCP retransmits) or veth alternative.
### Task 4: chaos rows
Acceptance #11 (kill holder uplink, floor frees in grace) + #12 (idle peers zero RTP payload — tcpdump byte count) + E6 worker kill (real recovery time measurement replaces the 2-4s estimate) + E7 restart drill.
### Task 5: sign-off + ledger
12-item acceptance table completed with evidence links; deviations logged; status.md Phase-1 → PoC-complete; retro (lesson-review) scheduled.

**Review Focus:** clock-offset between LAN machines (relative deltas only) · TURN credential expiry mid-session (1h boundary) · mobile browser background tab behavior (expected degradation, documented not fixed) · certificate trust on field devices (self-signed distribution) · netem command leakage to SSH session (separate console).
