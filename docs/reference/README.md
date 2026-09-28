# docs/reference Index

Per convention C2 (Diataxis split, mirroring DeskServo/MediaServo layout): **living references**
here at `reference/` root mirror product modules and are updated in place; **research archives**
under `research/<domain>/` are frozen one-off dossiers — never rewritten, new findings go into new dated files.

## Living references

None yet — this folder registers API/config/ops handbooks as product modules land (e.g. generated `signaling-protocol-schema.md`). DESIGN documents go to `../modules/` (C2).

## Research archives

### `research/ptt/` — floor/PTT domain & market
| Path | Compiled | Scope | Status |
|---|---|---|---|
| [research/ptt/standards-ptt-mcptt.md](research/ptt/standards-ptt-mcptt.md) | 2026-09-28 | OMA PoC / 3GPP MCPTT floor semantics -> TalkServo mapping; TETRA/DMR; B-Trunco | archived (frozen) |
| [research/ptt/commercial-ptt-products.md](research/ptt/commercial-ptt-products.md) | 2026-09-28 | commercial PTT feature matrix + product lessons + differentiation | archived (frozen) |
| [research/ptt/ptt-landscape.md](research/ptt/ptt-landscape.md) | 2026-09-28 | 7 OSS PTT peers + benchmark + new sweep candidates | archived (frozen) |
| [research/ptt/github-sweep-ptt.md](research/ptt/github-sweep-ptt.md) | 2026-09-28 | PTT keyword-family GitHub saturation sweep (137 repos, coverage claims) | archived (frozen) |
| [research/ptt/github-sweep-ptt-standards.md](research/ptt/github-sweep-ptt-standards.md) | 2026-09-28 | OMA/MCPTT/B-Trunco/SIP/murmur sweep — negative results registry | archived (frozen) |

### `research/media/` — media engine & audio pipeline
| Path | Compiled | Scope | Status |
|---|---|---|---|
| [research/media/media-stack-alternatives.md](research/media/media-stack-alternatives.md) | 2026-09-28 | WebRTC engine comparison + sweep findings (str0m/kraken/sfu crate) | archived (frozen) |
| [research/media/oss-voice-infrastructure.md](research/media/oss-voice-infrastructure.md) | 2026-09-28 | Mumble/LiveKit/mediasoup/Janus/Jitsi + Rust SFU inventory; arbitration precedents | archived (frozen) |
| [research/media/audio-processing-stack.md](research/media/audio-processing-stack.md) | 2026-09-28 | Opus FEC/PLC, NetEQ vs PTT churn, 3A crate landscape (crates.io-verified) | archived (frozen) |
| [research/media/github-sweep-webrtc.md](research/media/github-sweep-webrtc.md) | 2026-09-28 | WebRTC SFU/engine family saturation sweep (230 repos, ranked PoC bases) | archived (frozen) |

### `research/ui/` — product surface patterns
| Path | Compiled | Scope | Status |
|---|---|---|---|
| [research/ui/ptt-ui-patterns.md](research/ui/ptt-ui-patterns.md) | 2026-09-28 | Discord/Zello/Mumble/talktome/openPTT... UI patterns: floor viz, PTT affordances, dispatch IA, gap ledger | archived (frozen) |
| [research/ui/admin-ui-patterns.md](research/ui/admin-ui-patterns.md) | 2026-09-28 | LiveKit/Daily/Janus/talktome/MediaServo admin surfaces; object spine; PoC/Alpha split | archived (frozen) |

### `research/tooling/` — dev-toolchain references
| Path | Compiled | Scope | Status |
|---|---|---|---|
| (pending P0-1) `pixi-task-migration-notes.md` | — | MediaServo pixi/scripts patterns evaluated live during D11 analysis; formalize when scripts ship | planned |

### `research/internal/` — self-reviews of this project's own design
| Path | Compiled | Scope | Status |
|---|---|---|---|
| [research/internal/architecture-review-floor.md](research/internal/architecture-review-floor.md) | 2026-09-28 | floor/wire adversarial review vs MCPTT/Mumble/LiveKit (13 findings) | archived (frozen) |
| [research/internal/architecture-review-media.md](research/internal/architecture-review-media.md) | 2026-09-28 | mediasoup trust/uplink/mechanism review vs official docs+crate source (11) | archived (frozen) |
| [research/internal/architecture-review-ops.md](research/internal/architecture-review-ops.md) | 2026-09-28 | TLS/JWT/TURN/health/supply-chain (live crate stats) (10) | archived (frozen) |
| [research/internal/architecture-review-client.md](research/internal/architecture-review-client.md) | 2026-09-28 | browser matrix/Electron/token-UX vs BCD+Electron+LiveKit docs (12) | archived (frozen) |
| [research/internal/architecture-review-consolidated.md](research/internal/architecture-review-consolidated.md) | 2026-09-28 | lead merge: 46 raw → 21 unique ranked + held-up list + PIT-4 | **actionable index** |
