---
name: think-before-act
description: "Meta-constraint: investigate first, then act; present options and wait for user approval, never act recklessly. Use BEFORE any non-trivial action (debug/test/implement/refactor/fix/config/upgrade). Especially when facing silent failures, runtime errors not caught by compiler, or behavior that contradicts documentation."
---

# think-before-act: investigate → propose → approve → (team review) → execute

> **Ledger note (doc-audit 2026-09-28, PIT-3)**: bare `PIT-{n}` / `C{n}` / `D{nn}` identifiers in this file cite the **MediaServo ledger** (sister project), not this repo — TalkServo's own ids live in `.agents/memorys/` (C1-C2, D1-D11, PIT-1-PIT-3).


> Never act recklessly.

## Trigger conditions

Any **non-trivial** operation: debugging, testing, implementing, refactoring, fixing, config changes, upgrades.
Especially when failure patterns are abnormal: silent failures, runtime errors the compiler cannot catch, behavior contradicting documentation.

---

## Decision gate (ask yourself before every operation)

1. **"What does the official source recommend?"** → If you don't know, look it up. Don't guess.
2. **"Has this project encountered this before?"** → Check the project knowledge base (.agents/memorys/ / git log).
3. **"Has the community solved this?"** → Search issues / forums / StackOverflow (search with the exact error message).
4. **"Does the user agree with this approach?"** → Present options and wait for approval. Don't change anything directly.
5. **"Why did the previous attempt fail?"** → If you can't explain it, don't try the next one.

---

## Tiers

| Tier | Criterion | Process |
|------|------|------|
| **Trivial** | No interface/build/dependency changes, reversible, single file, no side effects | Do it directly + one-line explanation |
| **Standard** | Changes interface/build/dependency/multiple files, irreversible, framework-level issue | Phase 1→2→3 |
| **Urgent** | User explicitly declares "urgent / fix first, report later" (production outage, data loss) | Do it first + report afterwards |

---

## Phase 1: Investigate (mandatory for the standard tier)

### 5 knowledge sources (by priority)

1. **Project history**: .agents/memorys/decisions.md / pitfalls.md / git log
2. **Language/toolchain native docs**: cargo doc / rustdoc / --help / man
3. **Framework/tool official docs**: look for the recommended practice, not just any search
4. **Community experience**: GitHub issues / StackOverflow / forums (search the exact error message)
5. **Similar projects**: how open-source implementations with the same stack do it

### Extra checklist for test scenarios

- [ ] Project test conventions (framework? AAA? naming? coverage?)
- [ ] Spec/contract documents → extract test scenarios
- [ ] Existing test patterns (how is the same module tested? fixtures?)

### Output

Investigation summary: what was found, what the official source recommends, what the project conventions are.

---

## Phase 2: Propose options (standard tier, no automatic execution)

### Format

```
Option A: [description] - pros / cons / impact scope
Option B: [description] - pros / cons / impact scope
Recommendation: [X], rationale: [...]
```

### Code changes must be listed

- Which files will be modified
- Summary of changes (not the full diff)
- Potential risks
- Rollback method

### Approval

Wait for user confirmation via the `question` tool.
- Silence / timeout / "继续" ≠ approval <!-- c1:allow-zh -->
- The user must give an explicit affirmative response
- Partial approval ("do A, not B") → execute only the approved part

### Complex changes: team review

For complex changes (build/architecture changes, framework-level issues, impact on more than 5 files),
after user approval and before execution, add a team review step:

```
1. team_create: create a review team (3 perspectives)
2. team_task_create: assign review tasks
3. team_send_message: send the plan draft
4. Collect review feedback → revise the plan
5. User confirms the revision → proceed to Phase 3
```

CRITICAL items found in review must be fixed. LOW items can be recorded as technical debt.

---

## Phase 3: Execute (user has approved)

- Execute the option the user chose
- Verify at every step (don't accumulate unverified changes)
- On failure: roll back + report, **don't automatically try option B** (return to Phase 1 for more investigation)

---

## Core rules

1. **No new information = no new attempt.** Retrying with different parameters after a failure, without new diagnostic information, is reckless.
2. **Test the right layer.** Unit (logic) / integration (boundaries) / E2E (user flows) / acceptance (business): different layers catch different failures. If unit tests pass but the feature is broken, you tested the wrong layer.
3. **Trivial code needs no tests.** One line, no branches, no side effects → YAGNI applies to tests too.

---

## Test layer quick reference

| Layer | Catches | Rust tooling |
|------|----------|-----------|
| Unit | Logic errors | `cargo test` |
| Integration | Interface mismatches | `cargo test --test '*'` |
| E2E | Broken user paths | Playwright / Python WS scripts |
| Acceptance | Unmet requirements | Manual verification / pixi run test-sfu |

---

## Forbidden list (verify Y before doing X)

| Want to do | Verify first |
|------|--------|
| Change config/build | Checked the officially recommended configuration approach |
| Retry after a failure | Have new diagnostic information (not luck with different parameters) |
| Edit generated files / build artifacts | Confirmed the change won't be overwritten on the next build |
| Delete something you don't understand | Confirmed it isn't depended on by other modules |
| Edit files directly | User has approved the plan |
| Change dependencies/pixi.toml in production | Local verification passed first |

---

## Relationship to existing skills

| Skill | Relationship |
|------|------|
| `lesson-memory` (C9) | think-before-act checks past lessons; lesson-memory records new ones |
| `lesson-review` | think-before-act is prevention; lesson-review is retrospective |
| `systematic-debugging` | think-before-act covers "investigate before fixing"; debugging covers "how to fix" |
| `verification-before-completion` | think-before-act covers "before starting"; verification covers "after finishing" |

## Cross-ecosystem examples

| Ecosystem | Typical "investigate before acting" scenario |
|------|----------------------|
| Rust | Lifetime error → read the relevant rustbook chapter first, don't sprinkle `.clone()` |
| Python | Dependency conflict → check pip/uv resolution rules first, don't repeatedly `pip install` |
| Go | Interface not satisfied → read the godoc interface contract first, don't change signatures blindly |
| JS/TS | Duplicate modules → check the bundler's official recommendation first, don't tweak alias randomly |
| Java | Version conflict → run `mvn dependency:tree` first, don't manually exclude |
| K8s | Pod crash → check official troubleshooting + `kubectl describe` first, don't randomly edit yaml |

---

## Mapping to Karpathy's four principles

This project's think-before-act skill aligns naturally with Andrej Karpathy's four engineering philosophy principles:

| Karpathy principle | This skill's counterpart | Explanation |
|--------------|-----------|------|
| **Think Before Coding** | Phase 1: investigate + 5 knowledge sources | Check project history, official docs, and community experience first; don't start coding from a guess |
| **Simplicity First** | Core rule #3 + tiers (trivial/standard) | Prefer simple options, YAGNI, no over-engineering |
| **Surgical Changes** | Phase 2: propose options + forbidden list | Make precise changes, list the impact scope, don't touch unrelated code |
| **Goal-Driven Execution** | Phase 3: execute + decision gate | Execute the approved plan, verify every step, stay on track |

> **Community reference**: [multica-ai/andrej-karpathy-skills](https://github.com/multica-ai/andrej-karpathy-skills) turns Karpathy's engineering philosophy into executable AI skills, consistent with this skill's design philosophy.
