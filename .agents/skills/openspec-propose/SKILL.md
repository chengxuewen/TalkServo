---
name: openspec-propose
description: >-
  Propose a new change for TalkServo with structured artifacts (proposal, design,
  tasks). Generates docs/plans/<name>/proposal.md + design.md + tasks.md.
  Use when the user describes what they want to build and needs a complete proposal ready for implementation.
license: MIT
compatibility: Designed for Claude Code, GitHub Copilot, and similar agents.
disable-model-invocation: false
metadata:
  author: openspec
  version: "2.0"
  category: workflow
  project: TalkServo
---

# OpenSpec Propose — TalkServo
> **⚠️ Porting warning (doc-audit 2026-09-28)**: this skill's process skeleton is reusable, but its project-specific examples were ported from the DeskServo/MediaServo family and **do not describe TalkServo** (HAL/FlatBuffers/Studio/Zenoh references). TalkServo facts: docs/architecture.md + docs/modules/ + ledger D1-D11. Until this skill is re-authored for TalkServo, treat any embedded architecture detail as legacy illustration.


Create a structured change proposal for TalkServo. Produce three artifacts that together answer
"what are we building, how does it fit, and what's the plan?"

When ready to implement, follow with `/openspec-apply`.

---

**Input**: The user describes a feature, fix, or refactor. Do not start without a feature description.

---

## Steps

### 1. Confirm the change name

Ask: "What should we call this change? (kebab-case, e.g. `add-signal-monitoring`)"

**DO NOT auto-generate without asking.** Validate: lowercase letters, digits, hyphens only.

### 2. Gather context

Before writing any artifact, understand the existing surface area:

#### a. Read relevant specs

Search `openspec/specs/` for specs related to the change by module prefix:
`hal-type-system`, `hal-qos`, `hal-protocol`, `hal-config-barrier`.
Read every spec whose module overlaps. Note if no relevant spec exists.

#### b. Read project memory

- `.agents/memorys/status.md` — current phase, module status, known gaps
- `.agents/memorys/decisions.md` — D1-D11 architecture decisions
- `.agents/memorys/pitfalls.md` — known sharp edges (mediasoup worker lifecycle lineage, porting referential-integrity PIT-3, subagent-verification PIT-1)
- `.agents/memorys/conventions.md` — C1 English-artifacts, C2 docs-tiering

#### c. Assess affected layers

| Layer | Location | When affected |
|-------|----------|---------------|
| **Domain core** | `crates/talkservo-core/` | FloorState/apply(), wire enums, priority rules |
| **SFU host** | `crates/talkservo-sfu/` | mediasoup supervisor, Router/transport, apply_floor mapping |
| **Server** | `crates/talkservo-server/` | WS signaling, rooms, JWT, timers |
| **Web** | `web/` | SPA screens, PTT affordances, mediasoup-client |

#### d. Assess affected transports

| Media backend (feature) | Stage | Purpose |
|-----------|-------|---------|
| **sfu-mediasoup** | PoC default | mediasoup worker (Linux x86_64) |
| **stub-media** | macOS/CI check | signaling-only fallback (explicit) |

Most PoC changes target talkservo-core + talkservo-server (modules/02/03).

### 3. Create the proposal directory

```bash
mkdir -p docs/plans/<change-name>
```

### 4. Write proposal.md

Create `docs/plans/<change-name>/proposal.md` with these sections:
- **What** — 2-4 sentences, specific
- **Why** — problem, use case, gap
- **Scope** — in scope / out of scope
- **Layers Affected** — checklist: HAL Core / talkservo / FlatBuffers / Studio
- **Transports Affected** — talkservo: yes/no/partial, amw_zenoh: yes/no/partial
- **Existing Specs** — list `openspec/specs/<name>.md` with one-line description each
- **New Specs Needed** — list or "None"
- **Risks** — 2-4 bullet points (thread safety, FFI, build, interop)
- **Success Criteria** — how we know it's done
- **References** — links to issues, design docs, external references

### 5. Write design.md

Create `docs/plans/<change-name>/design.md` with these sections:
- **Architecture** — ASCII diagram or text description showing modules, data flow, ownership
- **Files to Touch** — Create / Modify / Delete sub-tables with file paths and purpose
- **Data Flow** — critical path from entry to exit (Signal: write→store→callback; RPC: invoke→dispatch→result)
- **Integration Points** — wire-contract boundary (modules/02), Sfu trait boundary (modules/03), floor-event flow server→sfu
- **Rust Specifics** — apply() purity preserved (no I/O in core), generation monotonicity, feature gates (D5/D6), thiserror types
- **Error Handling** — modules/05 matrix (E1-E10) + DenyReason enum; every log line room/peer/generation
- **Testing Strategy** — checklist per architecture §4: core unit, stub-media integration, Playwright e2e, netem harness
- **Dependencies** — new cargo/npm deps (or "None")

### 6. Write tasks.md

Create `docs/plans/<change-name>/tasks.md`. Tasks must be **atomic, ordered, independently testable** — each produces one verifiable result. Structure in phases:

```markdown
# Tasks: <Change-Name>

## Phase 1: Foundation

- [ ] **Add `<type/fn>` to talkservo-core**
  - File: `crates/talkservo-core/src/<module>.rs`
  - Verify: `cargo check -p talkservo-core`

- [ ] **Implement for server/sfu**
  - File: `crates/talkservo-{server,sfu}/src/<file>.rs`
  - Verify: `cargo check -p talkservo-server --no-default-features --features stub-media`

## Phase 2: Media & Wire

- [ ] **Update wire contract / RtpParameters mapping** (if needed)
  - File: `crates/talkservo-core/src/wire.rs` + `web/src/wire.ts`
  - Verify: serde roundtrip test + modules/02 table update

## Phase 3: Tests

- [ ] **Add Rust unit tests** (AAA pattern)
  - File: same as implementation
  - Verify: `cargo test -p talkservo-core`

- [ ] **Add integration tests**
  - File: `tests/<name>_test.rs`
  - Verify: `cargo test --test <name>_test`

- [ ] **Run pixi gate** (`pixi run lint && pixi run test`)
  - Verify: `./scripts/qa/qa-fast.sh` (5 gates: test/clippy/fmt/deny/unwrap)

## Phase 4: Documentation & Cleanup

- [ ] **Write/update spec file**
  - File: `openspec/specs/<name>.md` (SDD format: ID→precondition→operation→expected→edge cases)

- [ ] **Update project memory** (after implementation)
  - `.agents/memorys/status.md`, `decisions.md`, `pitfalls.md` as applicable
```

Adjust phases to fit the change: single-file fix → 3 tasks; multi-module feature → 15+ tasks across 5 phases.

### 7. Present and iterate

Display summary — change name, artifact list, line counts. Let user request changes, iterate until approved.

---

## File Path Conventions

| Purpose | Path |
|---------|------|
| talkservo-core | `crates/talkservo-core/src/` |
| talkservo-sfu | `crates/talkservo-sfu/src/` |
| talkservo-server | `crates/talkservo-server/src/` |
| Web SPA | `web/src/` |
| Design docs | `docs/modules/` |
| Specs | `openspec/specs/` |
| Plans | `docs/plans/<change-name>/` |
| Integration tests | `tests/` |

---

## TalkServo-Specific Guidelines

### Crate references

| Crate | Path | Type |
|---------|------|------|
| HAL Core | `crates/talkservo-common/` | Rust (traits, types, primitives) |
| amw-inproc | `crates/talkservo/` | Rust (HAL Transport/Discovery in-process) |
| HAL FlatBuffers | `crates/hal-flatbuffers/` | Rust + .fbs schemas |
| Studio | `apps/studio/` | Tauri + React + TypeScript (D21) |

### Build commands

```bash
cargo build                                    # Full build
cargo build --package talkservo-common --package talkservo  # HAL-only
cargo test                                     # Debug build + tests
./scripts/qa/qa-fast.sh                        # QA fast gate (5 checks)
```

### Rust conventions

- Rust stable (edition 2024, rust-toolchain.toml), ownership, borrowing, traits
- Floor model per D1: apply() pure transitions, generation ordering
- Media per D6: mediasoup crate 0.24, feature-gated (stub-media fallback)
- Wire discipline: serde snake_case single enum (modules/02)
- No async/IO in talkservo-core (D5)
- Config via YAML → FlatBuffers (D24)

---

## Guardrails

- **Always ask for the change name** — do not generate one without user confirmation
- **Read specs before proposing** — ignoring existing SDD contracts is waste
- **Layer assessment must be explicit** — "maybe affects FlatBuffers" is not acceptable; decide and document
- **Transport assessment must be explicit** — talkservo-only? amw_zenoh? Both? Document the split
- **Tasks must be atomic** — each task produces one verifiable result (compiling code, passing tests)
- Always reference actual TalkServo file paths and crate names
- If context is critically unclear, ask — but prefer reasonable decisions to keep momentum
- If a proposal with that name already exists, ask to continue or create new
- Do NOT propose changes to `version.txt` — versioning is user-managed
- Do NOT propose changes to external dependencies — separate repositories
- Verify each artifact file exists after writing before proceeding
- Do NOT reference MODACS — fully de-MODACS-ized project (D3)
