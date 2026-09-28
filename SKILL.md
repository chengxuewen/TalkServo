# TalkServo Skills Registry

## Superpowers

General-purpose skills loaded via the `superpowers` plugin (brainstorming, systematic-debugging, TDD, verification-before-completion, etc.) — apply to every project.

## Project Skills

TalkServo-specific skills, located in `.agents/skills/` (22):

| Skill | Type | Notes |
|-------|------|-------|
| `think-before-act` | process | investigate → options → user approval before non-trivial actions |
| `doc-audit` | audit | 5-dimension documentation/architecture audit (team mode) |
| `ecosystem-scan` | audit | .agents/ infrastructure audit + community scan |
| `skill-router` | meta | intent → recommended skill combinations |
| `context-engineering` | meta | task → correct rule/skill routing across Rust/TS/FFI/protocol lanes |
| `lesson-review` | memory | batch session-review → extract lessons into memorys ledgers |
| `api-interface-design` | design | contract-first API design — ⚠️ body still carries MediaServo-lineage Component/Plugin trait examples; trust docs/modules/02-03 |
| `incremental-implementation` | process | thin vertical slices, per-crate commit discipline |
| `source-driven-development` | process | dependency decisions grounded in official docs |
| `code-simplification` | quality | Rust/TS over-engineering reduction (Chesterton gates) |
| `performance-optimization` | quality | latency tracing, bench regression |
| `security-hardening` | quality | OWASP + secrets sweep (absorbs `review-hardcode`) |
| `review-hardcode` | quality | thin alias into security-hardening Phase 1 |
| `test-harness` | testing | multi-language test skeleton generation, AAA + traceability |
| `browser-testing` | testing | Playwright/DevTools flows: PoC pages + (Alpha) admin |
| `ci-cd-automation` | ops | pipeline/tasks audit — ⚠️ assumes pixi/scripts that land at P0-1 |
| `book-to-skill` | tooling | documents → skill conversion |
| `openspec-propose` / `openspec-explore` / `openspec-apply-change` / `openspec-archive-change` / `openspec-sync-specs` | spec workflow | ⚠️ two of these carry DeskServo-lineage architecture illustrations (porting warnings in-file, doc-audit 2026-09-28); process skeleton usable, examples are not TalkServo facts — adjudication pending |

Skills are activated automatically by agents based on task context; routing table lives in `context-engineering`.
