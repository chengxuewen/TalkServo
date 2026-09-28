# .agents — AI Infrastructure Knowledge Base

**Parent:** ../../AGENTS.md

## OVERVIEW

Three-plane agent infrastructure for TalkServo: binding rules, task skills, and project ledgers. Ported from MediaServo 2026-09-28, then re-verified per PIT-3.

## STRUCTURE

```
.agents/
├── memorys/     # THE LEDGER (single source): conventions.md C1+, decisions.md D1+, pitfalls.md PIT-1+, status.md
├── rules/       # common/ (always-relevant) + per-language dirs (14 langs exist, only rust/ + 2 injected by default)
└── skills/      # 22 project skills, each SKILL.md + .skill_id; injected or on-demand
```

## WHERE TO LOOK

| Need | File |
|------|------|
| Which rules auto-inject per turn | ../../.opencode/opencode.json → `instructions[]` (18 files: memorys/2 + rules/common/*14 + rules/rust/*2) |
| New constraint / decision / pitfall | append to the matching ledger with its template format — executable check command mandatory |
| Skill list vs routing table | `context-engineering/SKILL.md` routing rows (re-check after any skill add/remove) |
| Skill self-test | `npx tsx scripts/verify-skills.ts` — script NOT yet ported; manual grep fallback below |

## CONVENTIONS (this subtree only)

- Ledger ids are foreign keys everywhere else in the repo: `PIT-{n}`, `C{n}`, `D{n}` in any file must resolve against these ledgers or carry an explicit "MediaServo" label (blanket "Ledger note" headers in skills do this).
- `rules/common/platform.md` + `docker.md`: MediaServo macOS-era posture with **TalkServo D11 calibration banner** — read the banner first; the bodies are conditional.
- `memorys/` files are English too (C1 covers them); status.md is the phase/decision snapshot — update in the same change as the ledger it reflects.
- Skills keep bilingual zero: descriptions English (they route agent behavior; C1).

## ANTI-PATTERNS

- Writing a new rule as prose without a runnable check command → rejected (lesson-memory standard).
- Back-editing skill bodies to "fix" MediaServo ids one-by-one instead of relying on the file-level Ledger note (PIT-3 drift trap).
- Duplicating ledger content into rules/skills prose — reference by id only.

## COMMANDS

```bash
grep -c "^## C" memorys/conventions.md; grep -c "^## D" memorys/decisions.md; grep -c "^## PIT-" memorys/pitfalls.md   # counts = ledger size truth
grep -rn "media-stack-alternatives\|docs/reference" skills/doc-audit/SKILL.md   # doc paths vs C2 layout spot-check
```
