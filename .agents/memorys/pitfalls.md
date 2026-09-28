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
- **Verification**: `grep -rPn '[\x{4e00}-\x{9fff}]' .agents .opencode .omo 2>/dev/null | grep -v 'c1:allow-zh'` (must be empty)
