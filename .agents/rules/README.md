# Rules

Rules are part of the `.agents/` directory, organized into a **common** layer plus **language-specific** directories. The tree below is the disk truth (2026-10-23); regenerate it with `ls .agents/rules/` when in doubt — this repo already paid for a README that described directories that never existed (full doc-audit 2026-10-23, F4).

```text
.agents/rules/
├── common/          # Language-agnostic principles — injected EVERY turn (14 files)
│   ├── agents.md            # agent orchestration + execution gate
│   ├── code-review.md       # review standards + severity gates
│   ├── coding-style.md      # KISS/DRY/YAGNI, naming, smells
│   ├── constraints.md       # stub: git-commit rules; points to docker/platform
│   ├── development-workflow.md  # feature workflow, TDD, commit format
│   ├── docker.md            # Docker/UDP/ICE constraints (D11/D26 calibration banner)
│   ├── edit-safety.md       # edit-tool rules 1-24 + recovery patterns
│   ├── hooks.md             # hook types + todo discipline
│   ├── lesson-memory.md     # when/where to write PIT/C/D entries
│   ├── patterns.md          # repository pattern, API envelope, skeletons
│   ├── performance.md       # model selection, context budget
│   ├── platform.md          # macOS/Linux posture (D11 calibration banner)
│   ├── security.md          # mandatory security checks
│   └── testing.md           # coverage, browser-lane rules, lane ratchets
├── rust/            # injected (2 of 5 files: coding-style.md, hooks.md)
├── typescript/      # injected (2 of 5: coding-style.md, testing.md) — for web/ + packages/client
└── web/             # injected (2 of 7: coding-style.md, testing.md) — SPA + e2e surface
```

Injection list = `.opencode/opencode.json` → `instructions[]` (22 entries as of 2026-10-23: memorys/2 + common/14 + rust/2 + typescript/2 + web/2).

- **common/** contains universal principles — no language-specific code examples.
- **Language directories** extend the common rules; the injected subset is what every turn sees. Files not injected (`rust/patterns.md`, `rust/security.md`, `rust/testing.md`, the remaining ts/web files) are pulled in by path-relevant tasks or read on demand — they still carry the standard `> This file extends [common/xxx.md](../common/xxx.md)` header.

Rules are project-local and read directly by the AI assistant from `.agents/rules/` — no copying to `~/.claude/` needed.

## Rules vs Skills

- **Rules** define standards, conventions, and checklists that apply broadly (e.g., "80% test coverage", "no hardcoded secrets").
- **Skills** (`skills/` directory) provide deep, actionable reference material for specific tasks.

Rules tell you *what* to do; skills tell you *how* to do it.

## Adding a Language or Domain

Only when the repo actually has source in that language (the "content earns its directory" anti-pattern applies here — 8 empty language dirs were deleted on 2026-10-23 for exactly this reason):

1. Create `.agents/rules/<lang>/` with files extending the common ones, each starting:
   ```
   > This file extends [common/xxx.md](../common/xxx.md) with <Language> specific content.
   ```
2. Wire the files you want injected into `.opencode/opencode.json` → `instructions[]`.
3. Update this README's tree.

## Rule Priority

When language-specific rules and common rules conflict, **language-specific rules take precedence** (specific overrides general — same layering model as CSS specificity / `.gitignore`).

Commit standards live in [common/development-workflow.md](common/development-workflow.md) ("Commit Message Format") — there is no separate `git-workflow.md`.
