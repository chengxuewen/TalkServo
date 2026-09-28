# TalkServo Documentation Index

> Updated: 2026-09-28 | Organization: Diataxis-inspired (specification / reference / research archive), mirroring the MediaServo docs layout.

## Core documents

| Document | Content | Status |
|----------|---------|--------|
| [whitepaper.md](whitepaper.md) | Positioning, Floor model overview, tech selection rationale, PoC roadmap, risks | planning draft 0.1 |
| [architecture.md](architecture.md) | Floor state machine, crate boundaries, signaling protocol, media pipeline, deployment modes, security, open questions | planning draft 0.1 |

## Research archive — `reference/research/`

One-time competitive and selection research (frozen dossiers); living references will register at `reference/README.md` as product modules land (convention C2).

| Document | Content | Status |
|----------|---------|--------|
| [research/standards-ptt-mcptt.md](reference/research/ptt/standards-ptt-mcptt.md) | OMA PoC / 3GPP MCPTT (TS 24.379/24.380/23.280) floor & priority semantics → TalkServo mapping; TETRA/DMR heritage; B-Trunco | verified (101 lines, team) |
| [research/commercial-ptt-products.md](reference/research/ptt/commercial-ptt-products.md) | Zello/Voxer/Rave/MCPTT-class/Hytera/Doro feature matrix, product lessons, open-core differentiation | verified (106 lines, team) |
| [research/github-sweep-ptt.md](reference/research/ptt/github-sweep-ptt.md) | PTT keyword-family exhaustive sweep (7 queries, 137 deduped repos, coverage claims, 5 new peers) | verified snapshot |
| [research/github-sweep-webrtc.md](reference/research/media/github-sweep-webrtc.md) | WebRTC SFU/engine sweep (230 repos; str0m/kraken/sfu-crate verified; ranked PoC bases) | verified snapshot |
| [research/github-sweep-ptt-standards.md](reference/research/ptt/github-sweep-ptt-standards.md) | OMA PoC/MCPTT/B-Trunco/SIP/murmur sweep — negative results + name-collision registry | verified snapshot |
| [research/ptt-landscape.md](reference/research/ptt/ptt-landscape.md) | 7 open-source PTT projects profiled + benchmark matrix + per-project lessons | verified snapshot |
| [research/oss-voice-infrastructure.md](reference/research/media/oss-voice-infrastructure.md) | Mumble/LiveKit/mediasoup/Janus/Jitsi/ion-sfu/Nextcloud Talk + Rust SFU inventory; arbitration precedents | verified (121 lines, team) |
| [research/audio-processing-stack.md](reference/research/media/audio-processing-stack.md) | Opus FEC/PLC/DTX (RFC 8215), NetEQ vs PTT churn buffering, 3A crate landscape verified on crates.io | verified (66 lines, team data) |
| [research/media-stack-alternatives.md](reference/research/media/media-stack-alternatives.md) | WebRTC engine comparison, Opus FEC mechanics, 3A routes | verified snapshot |

## Conventions (project-wide)

- All docs in English (C1, see `.agents/memorys/conventions.md`).
- Architecture docs cite decision IDs (`D{n}`) and pitfalls (`PIT-{n}`) from `.agents/memorys/`.
- Module-level docs (`docs/modules/NN-*.md`) are created when the corresponding crate lands — no speculative scaffolding.

## Planned documents (not yet written)

| Document | Trigger |
|----------|---------|
| `docs/modules/signaling-protocol.md` + OpenAPI/JSON schema | when `talkservo-core` compiles |
| `docs/modules/sfu-webrtc.md` | when `talkservo-sfu` exists |
| `docs/modules/sdk-contract.md` | when UniFFI/wasm bindings are generated |
| `docs/deployment.md` | Alpha split-services stage |
