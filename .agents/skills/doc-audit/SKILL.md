---
name: doc-audit
description: "TalkServo project documentation and architecture audit. Checks self-consistency, completeness, gaps, and optimization opportunities across architecture docs, design docs, and decision records in parallel. Presents findings interactively for item-by-item confirmation with the user, listing details, options with pros/cons, sources, impact, and recommendations. Supports team mode (large audits) and background agent mode (lightweight checks)."
---

# Document & Architecture Audit

A comprehensive audit of the TalkServo documentation system, covering document consistency, decision validation, agent infrastructure, and reference benchmarking.

**Philosophy**: Auditing is not fault-finding; it is debt clearing. Documentation debt is as dangerous as code debt.

---

## Entry: Audit Types

### `/doc-audit` (no arguments)
Pops up the audit type menu:

```
[1] Full Audit - all 5 dimensions (default)
[2] Decision Validation - decisions.md implementation status
[3] Document Consistency - arch <-> modules <-> sub-document cross-check
[4] Agent Infrastructure - .agents/ rules/skills/memory self-consistency
[5] Gap Optimization - security/reliability/ops scan
[6] Phase Audit - Phase progress vs actual state
```

### `/doc-audit full`
Starts the full audit directly, skipping the menu. Equivalent to `[1] Full Audit`.

### `/doc-audit quick-fix`
Only checks LOW/MEDIUM severity issues and auto-fixes them, skipping interactive review.

---

## Audit dimensions

### 1. Decision Validation
Verify that `decisions.md` D1-Dx are reflected in the architecture docs, implementation code, and status records.

**Core questions**:
- Are decision conclusions correctly reflected in architecture.md and module docs?
- Is there a contradiction where "the decision says A but the reality is B"?
- Do stale references in decisions need updating?
- **Decision freshness check**: has this decision been superseded by a later one? Is the tech stack version outdated?

### 2. Document Consistency
Verify cross-consistency across architecture.md <-> modules/ <-> README <-> AGENTS.md.

**Core questions**:
- Are descriptions of the same concept consistent? (crate count, Phase status, tech stack version)
- Do sub-documents duplicate the main document?
- Is Phase terminology ambiguous?
- Does crate naming follow the docs-structure convention (C2)?

### 3. Agent Infrastructure Audit (new)
Verify self-consistency among rules, skills, and memory under .agents/.

**Core questions**:
- Are conventions.md numbers sequential? Are C{n} cross-references valid?
- Are decisions.md numbers sequential? Is the D{n} order correct?
- Are pitfalls.md numbers sequential? Are PIT-{n} cross-references complete?
- Does the skill directory table (AGENTS.md) match the actual skill list?
- Does the task routing table (context-engineering) match the actual skill list?
- Do all referenced instructions in opencode.json exist?
- Is each skill's SKILL.md frontmatter complete?

### 4. Gap Optimization
Scan for missing key document/design sections.

**Core questions**:
- Security architecture: are PSK rotation, JWT expiry policy, and mediasoup transport security documented?
- Operations/observability: health checks, metrics export, log aggregation?
- Error model: SFU connection failure, encoding degradation, transport disconnect handling?
- Hardware baseline: minimum CPU/RAM, Docker resource config?
- Upgrade strategy: hot reload, crate version migration, config migration?

### 5. Phase Audit
Verify Phase progress vs documented claims vs code implementation.

**Core questions**:
- Does the Phase status in status.md match git log / test count?
- Are the decision Phase labels in decisions.md accurate?
- Does the code implementation match the completion status claimed in docs?

---

## Audit modes

### A. Team mode (recommended for large audits)
3+ large documents -> `team_create` 4-6 members in parallel.

```
team_create(inline_spec={
  name: "doc-audit",
  members: [
    { name: "decision-validator", category: "deep", prompt: "<Decision Validation Core Questions>" },
    { name: "consistency-checker", category: "deep", prompt: "<Document Consistency Core Questions>" },
    { name: "agent-auditor", category: "deep", prompt: "<Agent Infrastructure Core Questions>" },
    { name: "gap-optimizer", category: "deep", prompt: "<Gap Optimization Core Questions>" }
  ]
})
```

**Conductor rules** (dispatcher behavior):
- Immediately report to user on startup: "Starting N-way parallel audit, estimated 3-5 minutes"
- Only do "non-overlapping work" before all audits complete
- After all complete: **deduplicate and merge** (same issue found by 2+ dimensions -> merge into 1 item)
- Sort by severity: CRITICAL -> HIGH -> MEDIUM -> LOW
- Timeout handling: any lane with no output after 10 minutes -> mark as "timed out"
- Conflict handling: dimension A says X, dimension B says Y -> flag for human review

### B. Background agent mode (lightweight audit)
Few documents -> `task(category="deep", run_in_background=true)` x N in parallel.

### C. Single-threaded mode
Very small scope -> use Read/Grep directly, no subagents.

---

## Interactive review: finding format

**Item-by-item review**, each presented using the `question()` tool.

```markdown
## 🔴/🟠/🟡/🔵 [ID]: [Title]

### Details
| Source | Location | Content |
|--------|----------|---------|
| Doc A | Line X | ... |
| Doc B | Line Y | ... |

### Options
| Option | Pros | Cons |
|--------|------|------|
| A. [Option name] | ... | ... |
| B. [Option name] | ... | ... |

### Recommendation
[Option X]. [Rationale]
```

Options: accept recommended / choose other / skip / custom
Progress: `[N of M]`

---

## Workflow

### Phase 1: Startup
1. Confirm audit scope and type
2. Choose mode (team/background/single-threaded)
3. Report: "Starting N-way parallel audit"

### Phase 2: Merge
1. Deduplicate: same issue from multiple sources -> merge
2. Sort: CRITICAL -> HIGH -> MEDIUM -> LOW
3. Cross-validate: 2+ dimensions agreeing -> boost priority

### Phase 3: Interactive review
Item-by-item review with question() interactive confirmation.

### Phase 4: Fix
1. Create todo list
2. Follow dependency order: fix decisions first -> then docs -> finally status
3. Verify after each edit

### Phase 5: Report
```
Audit complete - [date]
Audit type: [full/decision/consistency/agent-infra/gap/phase]
Total findings: N | Fixed: M | Skipped: K
Next recommendation: [problem-dense areas]
```

---

## Severity criteria

| Severity | Trigger | Blocking? |
|----------|---------|:---------:|
| 🔴 CRITICAL | Doc contradiction causes implementation error / decision overturned / core API missing | ✅ |
| 🟠 HIGH | Stale reference / Phase ambiguity / duplicate docs / numbering gaps | ⚠️ |
| 🟡 MEDIUM | Wording difference / example conflict / missing but non-blocking for current phase | ❌ |
| 🔵 LOW | Format inconsistency / missing reference / unconfirmed marker | ❌ |

## Recommended audit frequency
- After every D# decision change: `/doc-audit decisions`
- Before Phase transitions: `/doc-audit full`
- Weekly during development: `/doc-audit full`
- After major doc changes: `/doc-audit consistency`

---

## Community references

| Precedent | Pattern adopted |
|-----------|----------------|
| [large-codebase-audit](https://github.com/MJWNA/large-codebase-audit-skill) | 9-surface AI-layer audit, aligned with Anthropic best practices |
| [claude-ecosystem](https://github.com/melodic-software/claude-code-plugins) | Meta-skill architecture, 16 audit agents |
| [agent-self-audit](https://github.com/Xxt-XN/agent-self-audit) | Dual-layer design, 13 checks, auto-upgrade |

---

## Division of labor with ecosystem-scan

| Dimension | ecosystem-scan | doc-audit |
|-----------|:---:|:---:|
| Agent infrastructure (skills/rules/MCP) | ✅ Specialty | ✅ Dimension 3 |
| External community comparison | ✅ Core capability | ❌ |
| Internal document consistency | ❌ | ✅ Specialty |
| Decision validation | ❌ | ✅ Specialty |
| Security audit gate | ✅ Full mode | ❌ |
