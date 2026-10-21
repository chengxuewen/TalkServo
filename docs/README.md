# TalkServo Documentation Index

> Updated: 2026-10-19 | Organization: Diataxis-inspired (specification / reference / research archive), mirroring the MediaServo docs layout.

## Core documents

| Document | Content | Status |
|----------|---------|--------|
| [whitepaper.md](whitepaper.md) | Positioning, Floor model overview, tech selection rationale, PoC roadmap, risks | planning draft 0.1 |
| [architecture.md](architecture.md) | MASTER: overview, module index, open questions, acceptance, sister-project relation | living (thin by design) |
| [modules/01-09](modules/) | design docs: floor model · signaling protocol · components · media pipeline · error model · deployment&security · sdk strategy · web ui · dev toolchain — crate boundaries, floor state machine, wire contract, media pipeline, deployment modes, security | living (updated per landed slice) |

## Research archive — `reference/research/`

One-time competitive and selection research (frozen dossiers); living references will register at `reference/README.md` as product modules land (convention C2).

| Document | Content | Status |
|----------|---------|--------|
| [research/standards-ptt-mcptt.md](reference/research/ptt/standards-ptt-mcptt.md) | OMA PoC / 3GPP MCPTT (TS 24.379/24.380/23.280) floor & priority semantics → TalkServo mapping; TETRA/DMR heritage; B-Trunco | verified (101 lines, team) |
| [research/commercial-ptt-products.md](reference/research/ptt/commercial-ptt-products.md) | Zello/Voxer/Rave/MCPTT-class/Hytera/Doro feature matrix, product lessons, open-core differentiation | verified (106 lines, team) |
| [research/ui/ptt-ui-patterns.md](reference/research/ui/ptt-ui-patterns.md) | mainstream voice/PTT UI patterns + floor-viz affordances + gap ledger | verified snapshot |
| [research/ui/admin-ui-patterns.md](reference/research/ui/admin-ui-patterns.md) | RTC admin console conventions + TalkServo admin v1 candidate list | verified snapshot |
| [research/ui/console-feature-checklist-2026-10-19.md](reference/research/ui/console-feature-checklist-2026-10-19.md) | mainstream admin/dispatch console feature lists w/ source URLs (LiveKit, Jitsi, Discord, Mumble, PushComm, Zello, MCPTT) + WebRTC SPA testing norms | verified snapshot 2026-10-19 |
| [research/github-sweep-ptt.md](reference/research/ptt/github-sweep-ptt.md) | PTT keyword-family exhaustive sweep (7 queries, 137 deduped repos, coverage claims, 5 new peers) | verified snapshot |
| [research/github-sweep-webrtc.md](reference/research/media/github-sweep-webrtc.md) | WebRTC SFU/engine sweep (230 repos; str0m/kraken/sfu-crate verified; ranked PoC bases) | verified snapshot |
| [research/github-sweep-ptt-standards.md](reference/research/ptt/github-sweep-ptt-standards.md) | OMA PoC/MCPTT/B-Trunco/SIP/murmur sweep — negative results + name-collision registry | verified snapshot |
| [research/ptt-landscape.md](reference/research/ptt/ptt-landscape.md) | 7 open-source PTT projects profiled + benchmark matrix + per-project lessons | verified snapshot |
| [research/oss-voice-infrastructure.md](reference/research/media/oss-voice-infrastructure.md) | Mumble/LiveKit/mediasoup/Janus/Jitsi/ion-sfu/Nextcloud Talk + Rust SFU inventory; arbitration precedents | verified (121 lines, team) |
| [research/audio-processing-stack.md](reference/research/media/audio-processing-stack.md) | Opus FEC/PLC/DTX (RFC 8215), NetEQ vs PTT churn buffering, 3A crate landscape verified on crates.io | verified (66 lines, team data) |
| [research/media-stack-alternatives.md](reference/research/media/media-stack-alternatives.md) | WebRTC engine comparison, Opus FEC mechanics, 3A routes | verified snapshot |

## Living references — `docs/reference/` (C2)

| Document | Content | Status |
|----------|---------|--------|
| [reference/server-api.md](reference/server-api.md) | implemented server surface: routes, J-sequence, floor/media behaviors, config keys, honest gaps | living (plan-2) |

## Conventions (project-wide)

- All docs in English (C1, see `.agents/memorys/conventions.md`).
- Architecture docs cite decision IDs (`D{n}`) and pitfalls (`PIT-{n}`) from `.agents/memorys/`.
- `docs/modules/NN-*.md` = design docs (updated in place as implementation lands). `docs/reference/` = living API/config handbooks + frozen research archives under `reference/research/<domain>/`. (C2)

## Planned documents (not yet written)

| Document | Trigger |
|----------|---------|
| `docs/reference/signaling-schema.md` — OpenAPI/JSON-schema generated from `talkservo-core` wire types | when talkservo-core compiles |
| `docs/reference/deployment.md` — split-services rollout, capacity, upgrade/rollout policy (OQ gap fix) | Alpha |
| `docs/reference/config-handbook.md` — live mirror of the modules/06 §config table | first config drift |

All design docs already live in `docs/modules/01-09` (C2 tiering).

## Implementation plans — `docs/plans/` (C2 working tier)

| Plan | Scope | Status |
|------|-------|--------|
| [plans/2026-09-28-p0-workspace-core.md](plans/2026-09-28-p0-workspace-core.md) | P0-1 toolchain + talkservo-core first slice | **EXECUTED 2026-09-29** (4 gate commits) |
| [plans/2026-09-28-plan2-server-sfu.md](plans/2026-09-28-plan2-server-sfu.md) | signaling server + SFU host | skeleton (expand at execution) |
| [plans/2026-09-28-plan3-web-client-sdk.md](plans/2026-09-28-plan3-web-client-sdk.md) | packages/client + web SPA | **EXECUTED 2026-09-29** (T1-T4 done; e2e browser-gated) |
| [plans/2026-09-28-plan4-deployment-acceptance.md](plans/2026-09-28-plan4-deployment-acceptance.md) | public deployment + 12-item acceptance | skeleton |
| [plans/2026-10-08-plan5-admin-surface.md](plans/2026-10-08-plan5-admin-surface.md) | admin minimal surface (/admin/api + /a console) per D18 | **EXECUTED 2026-10-08** (S0/S1/S2; e2e 4/4 incl. click-through) |
| [plans/2026-10-08-plan6-tooling-ship-shell.md](plans/2026-10-08-plan6-tooling-ship-shell.md) | ship tooling: talkservo.sh + oxmgr instance facade + YAML config (D19) | **EXECUTED 2026-10-08** (shell + facade + vendored oxmgr + instance model) |
| [plans/2026-10-19-alpha-wave1-trust-control.md](plans/2026-10-19-alpha-wave1-trust-control.md) | Alpha wave-1: audit + room authority + tiers + intervention kit (D23) | DRAFT (review pending) |
| [plans/2026-10-21-alpha-wave2-realtime-control.md](plans/2026-10-21-alpha-wave2-realtime-control.md) | wave-2: /admin/ws AdminEvent push (separate contract, signaling wire untouched), Listen deferred (R-F14 + D13/#12, four prerequisites recorded) (D23 deferred family) | **ADJUDICATED R1-R5, awaiting go** |
| [plans/2026-10-19-plan8-polyorch-build-assembly.md](plans/2026-10-19-plan8-polyorch-build-assembly.md) | D25: PolyOrch build graph (lanes/gates/install), scripts thin, P1-P4 | **COMPLETE 2026-10-19** (P1-P4 landed; fresh-clone 4/4; CI on graph verbs) |
