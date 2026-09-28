---
name: lesson-review
description: "Batch session review: systematically extract lessons learned and write them to project memory. Use after long debugging sessions (>1h or >3 failed attempts), when user corrects multiple errors, after build fixes, on '总结经验'/'更新记忆'/'记录教训'/'回顾会话', or /lesson-review command."  # c1:allow-zh
---

# lesson-review: batch lesson harvesting

> **Ledger note (doc-audit 2026-09-28, PIT-3)**: bare `PIT-{n}` / `C{n}` / `D{nn}` identifiers in this file cite the **MediaServo ledger** (sister project), not this repo — TalkServo's own ids live in `.agents/memorys/` (C1-C2, D1-D11, PIT-1-PIT-3).


## Trigger conditions

- User says '总结经验' / '更新记忆' / '记录教训' / '回顾会话'  <!-- c1:allow-zh -->
- Long debugging session ends (>1h or >3 failed attempts)
- After user points out multiple errors
- After resolving a problem that took >30min
- `/lesson-review` command

## Relationship with rules and skills

```
think-before-act  ->  [Action]  ->  lesson-memory  ->  doc-audit
   (consult)                        (instant capture)   (periodic audit)
                    lesson-review <- batch gap-filling
```

| | `lesson-memory` rule | `lesson-review` skill |
|---|---|---|
| Timing | Instant (after every error, auto-triggered) | Batch (session end / user-triggered) |
| Method | Reflection | Interactive review |
| Granularity | Single lesson | Full session scan |
| Complement | First line of defense (no omissions) | Second line of defense (no misclassification + sorting) |

- `think-before-act` (consult): check existing lessons before acting, avoid repeating mistakes
- `lesson-memory` (C9 rule): **instantly auto-written** after every error, no user request needed
- `lesson-review` (this skill): **batch review** at session end, fill gaps, sort and archive
- `doc-audit` (audit): periodically check memory file self-consistency

## Flow

### Step 1: Scan session

Review which moments in this session triggered rules but may have been missed:

- Compilation/build failures
- >1 failed attempt before locating root cause
- User correcting approach/preferences
- Unexpected discoveries ("ah-ha moments", "I didn't expect that")
- Problems taking >30min
- Post-edit syntax damage (unbalanced braces, duplicate lines, etc.)

### Step 2: Extract one by one (5-question checklist)

For each finding, complete each item:

1. **What went wrong?** (symptom description)
2. **Why did it go wrong?** (root cause analysis)
3. **What's the correct approach?** (solution)
4. **How to prevent it?** (check command / constraint)
5. **Where to store it?** (per the storage target table)

### Step 3: Write

Write using the target file's template format.

- Important lessons -> full template (symptom + root cause + solution, three elements)
- Trivial fixes (1 line / no branch / no side effects) -> 1-line capture, no full template needed

### Step 4: Verify

- [ ] Every pitfall has a verify field (check command -- how to confirm it's fixed)
- [ ] Every convention can be verified by grep/lint
- [ ] No duplicates with existing entries (grep target file to confirm)
- [ ] decisions.md numbers are sequential (no gaps)
- [ ] Cross-references are correct (e.g. pitfalls referencing conventions)
- [ ] Mark high-frequency/high-cost lessons for potential CI gate escalation

## Output format

```markdown
## Session review summary -- YYYY-MM-DD

### Recorded (N items)
1. [Title] -> pitfalls.md
2. [Title] -> conventions.md
...

### Not recorded (no need)
- [Reason: one-time issue / already recorded / environment-specific]

### Suggested escalation
- [A lesson recommends adding a CI gate, reason: repeated 3+ times / took >1h]
```

## Storage targets (per project configuration)

Applicable to any project, just modify paths:

```
- Technical pitfalls -> {pitfall_log}      # TalkServo: .agents/memorys/pitfalls.md
- Dev conventions -> {conventions}         # TalkServo: .agents/memorys/conventions.md
- Architectural decisions -> {decisions}   # TalkServo: .agents/memorys/decisions.md
- Executable checks -> {checks}            # TalkServo: .agents/rules/common/edit-safety.md
- Test requirements -> {test_rules}        # TalkServo: .agents/rules/common/testing.md
- Security rules -> {security_rules}       # TalkServo: .agents/rules/common/security.md
- Rust coding -> {rust_style}              # TalkServo: .agents/rules/rust/coding-style.md
- Project status -> {status}               # TalkServo: .agents/memorys/status.md
```
