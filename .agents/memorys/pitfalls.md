# TalkServo Pitfalls

> Format (five-section):
> ```
> ## PIT-{n}: Title (date)
> - **Symptom**: [observed behavior]
> - **Root cause**: [root-cause analysis]
> - **Solution**: [correct approach]
> - **Verification**: [check command]
> ```
> Numbering starts at PIT-1 and increases consecutively. Every pitfall must map to an executable rule (see lesson-memory.md).
>
> Note: the documentation system was ported from MediaServo on 2026-09-28; its PIT-1~207 history is not migrated.
> MediaServo-domain pitfalls (libwebrtc/mediasoup/iceoryx2 etc.) are re-filed here only when they actually recur
> in this project, marked "sibling of MediaServo PIT-{n}".

## PIT-1: Subagent self-reports of "done" are untrustworthy; verify with grep against the filesystem (2026-09-28)
- **Symptom**: During the English-migration batch, 4 of 6 writing agents emitted complete "done" reports while repo-wide grep showed 0 changes landed on disk (CJK counts identical to the originals); another agent left edit-safety.md half-rewritten with garbage trailing content (229→123 lines).
- **Root cause**: Full-file rewrites of long files exhausted the agents' turns inside their thinking loop before any write tool call; resumed sessions contaminated by the failed pattern repeated it.
- **Solution**: After collecting any batch-rewrite results, first reconcile per-file violation counts with repo-level grep before deciding to resume or re-dispatch; re-dispatch with fresh sessions, one file per task, forcing the first tool call to be read/write; write critical files (those injected into instructions every turn) yourself.
- **Verification**: same command as the C1 check (conventions.md), which covers this ledger too

## PIT-2: Rewriting/squashing local history — use commit-tree, not rebase --root, when untracked runtime files collide (2026-09-28)
- **Symptom**: `git rebase --root -x "git commit --amend --reset-author"` aborted: "untracked files would be overwritten by merge" (`.omo/run-continuation/*.json` exist on disk, ignored but present; replaying commits that touch them conflicts). A follow-up `update-ref` appeared to fail because a stale terminal view still showed the old hash while HEAD had already moved.
- **Root cause**: rebase replays trees through the working directory; any on-disk untracked file inside a path the commits manage blocks checkout. Ignore rules do not protect against rebase checkout. Verification must read `git rev-parse HEAD`, not remembered output.
- **Solution**: For pure history rewrite (squash / author change) with a clean index, build the commit directly: `git write-tree` (or reuse an existing `<commit>^{tree}`) → `GIT_AUTHOR_*/GIT_COMMITTER_*` env → `git commit-tree` → `git update-ref refs/heads/main $new $old`. Never touches the worktree. Before `push --force`, confirm lost remote commits carry no unique content (`git log HEAD..origin/main` + `git diff --stat`).
- **Verification**: `git rev-list --count HEAD` (expected commit count); `git status --short` empty; `git ls-remote origin main` equals `git rev-parse HEAD`.

## PIT-3: Ported artifacts carry foreign-key references into a ledger that was reset (2026-09-28)
- **Symptom**: doc-audit found in ported skills/rules: 7 dangling C{n} refs (C4/C5/C6/C9/C13/C21), 5 refs to nonexistent PIT-{n}, one ACTIVE semantic clash (`[PIT-07]` = MediaServo's test-deletion lesson but reads against our PIT-1/2), ~40 unlabeled MediaServo PIT citations in edit-safety.md — and C2 itself was authored in Chinese, violating C1 from inside C1's own ledger.
- **Root cause**: the port/rename pass treated prose as language content, not as DATA containing foreign keys into a deliberately reset ledger. C/D/PIT citations are joins; reset invalidates all of them silently.
- **Solution**: on any cross-project port, script a referential-integrity pass: extract project-scoped identifiers (C/D/PIT ids, crate names, ports, doc paths), join against the target ledger, then re-anchor, delete, or label "MediaServo {id}". Never trust renamed prose.
- **Verification**: `grep -rn 'PIT-[0-9][0-9]\|C[1-9][0-9]?\b' .agents/skills/*/SKILL.md | grep -v MediaServo` cross-checked against `.agents/memorys` ids (audit join, rerun after any port).
