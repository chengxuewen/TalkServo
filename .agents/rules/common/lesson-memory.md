# Lesson Memory & Self-Correction Rule

> When the AI hits a mistake, a new problem, or a new lesson, it **proactively** writes the lesson into project memory and rule files.

## Trigger Conditions (any one triggers it)

| Trigger | Action |
|----------|------|
| Compile/build failure after editing code | Record in `pitfalls.md` + update `edit-safety.md` |
| Same class of error appears a 2nd time | **Immediately** add a rule to `edit-safety.md` or create a new rule file |
| Community solution already exists | Record in `pitfalls.md` (with community link) + update `decisions.md` |
| Tests pass but the feature does not work | Record in `pitfalls.md` + update `testing.md` with E2E requirements |
| Dependency version conflict | Record in `pitfalls.md` + update `edit-safety.md` |
| Patch/script breaks build artifacts | Record in `pitfalls.md` + update `edit-safety.md` |
| User points out a problem the AI missed | Record in `pitfalls.md` + analyze root cause + add a prevention rule |
| Root cause found only after >1 failed attempt | Record in `pitfalls.md` (failed attempts + correct path) |
| User corrects an approach/preference | Record in `conventions.md` or `decisions.md` |

## Write Targets

| File | Content | Format |
|------|---------|--------|
| `.agents/memorys/pitfalls.md` | Problem + cause + solution + verification + prohibition | Five-section (PIT-{n}) |
| `.agents/memorys/decisions.md` | Architecture/technical decision + rationale + references | `## D{N}: Title` |
| `.agents/memorys/conventions.md` | Development constraints/user preferences | C{n}: "constraint" + check command |
| `.agents/rules/common/edit-safety.md` | Enforceable check rules | `### N. Rule name` + check command + blocking condition |
| `.agents/rules/common/testing.md` | Test coverage requirements | Test type + run command + pass criteria |

## When to Write

1. **Write immediately**: record it the moment the problem is found, do not wait for the session to end.
2. **Verify after fixing**: after writing, verify the rule is executable (has concrete commands, not platitudes).
3. **Dedup check**: before writing, `grep` the target file to confirm the entry is not a duplicate.

## Rule Quality Standards

| Standard | Description |
|----------|-------------|
| **Executable** | Contains concrete shell commands or check steps, not "pay attention to XXX" |
| **Verifiable** | Has explicit pass/fail criteria |
| **Blocking** | States explicitly what to do when it fails |
| **Precedented** | Cites an error that actually happened |

## Anti-Patterns (Forbidden)

| Anti-Pattern | Correct Practice |
|--------------|------------------|
| "I'll be careful next time" | Write concrete check commands into the rule file |
| Record without preventing | Every pitfall must map to an executable rule |
| Rules that say "edit carefully" | Write `grep -c '{' file && grep -c '}' file` |
| Wait for the user to point it out | Record on discovery (immediately), don't wait for the user |

## Capture Checklist (ask yourself on every trigger)

1. **What went wrong?** (symptom)
2. **Why did it go wrong?** (root cause)
3. **What is the correct approach?** (solution)
4. **How to prevent it?** (check command / constraint)
5. **Where to store it?** (per the write-target table)

## Trivial-Fix Escape Hatch

1-line fix / no branches / no side effects → 1-line capture, full template not required.

## Pitfall Template (must be included when writing to pitfalls.md)

```markdown
## PIT-{n}: Title (date)
- **Symptom**: [observed behavior]
- **Root cause**: [root-cause analysis]
- **Solution**: [correct approach]
- **Verification**: [check command]
```

## Prevention Escalation (high-cost / high-frequency lessons)

- Repeats ≥2 times or cost >3 rounds to locate → add a CI gate or pre-commit hook
- Low frequency / low cost → record in pitfalls.md only
