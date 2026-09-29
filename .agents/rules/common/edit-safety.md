# Code Edit Safety

> **Reference note (2026-09-28, PIT-3)**: all `PIT-{n}`/date precedent citations below are **historical, from MediaServo's ledger** unless they say "this project". TalkServo's own pitfalls restart at PIT-1 (`memorys/pitfalls.md`). The rules themselves are re-verified as still-correct for this repo.

> **Target audience**: AI agents editing TalkServo source code.
> **Violation of these rules causes token waste from repeated fix cycles.**

## Tool Selection

| Change Size | Tool | Reason |
|-------------|------|--------|
| Rewrite entire function/file | `write` | Guarantees brace balance, no stale lines |
| ≤20 line single-location edit | `edit` | Minimal diff, safe for small changes |
| Structural pattern replacement | `ast_grep_replace` | Syntax-aware, preserves matching |
| Complex multi-file refactor | Delegate to subagent | Isolated context, verify independently |

## Forbidden Patterns

| Anti-Pattern | Why |
|--------------|-----|
| `sed` for code modification | Quote escaping errors, regex silent failures |
| Multiple sequential `edit` calls without re-reading | Line numbers drift, stale hash IDs |
| Deleting a line by replacing with empty `lines: []` and assuming brace count is still correct | May leave unbalanced braces |
| Appending `}` to "fix" an unclosed delimiter without counting braces first | Masks root cause, may create double-close |

## Verify Immediately

After EVERY code change (edit, write, or ast_grep_replace):

```
Rust:   cargo check -p <crate>       (5-15s)
TS/TSX: npx tsc --noEmit             (3-5s)
YAML:   docker compose config --quiet (compose file)
Shell:  bash -n <script>             (script)
```

### Batch `edit` arrays must be verified per operation (PIT-41)

When multiple replace operations touch adjacent regions, boundary lines can be misaligned (one operation may overlap the region another preserves). Run the format check after each edit call; when damaged → re-read the file to restore, don't keep stacking the fix.

If verification fails, STOP. Do NOT apply another edit on top. Instead:
1. `git diff` to see what changed
2. If the change is wrong, `git checkout -- <file>` to revert
3. Re-apply the fix correctly

## Brace Safety Checklist

Before marking any multi-line edit complete, verify:
- [ ] Every `{` has a matching `}` at each area
- [ ] Every `(` has a matching `)`
- [ ] Every `[` has a matching `]`
- [ ] No duplicate function definitions or closing braces
- [ ] `cargo check` / `tsc --noEmit` passes

## When to Delegate

Delegate to a `deep` category subagent when:
- The change touches 3+ files
- The change requires understanding cross-module dependencies
- The change is beyond your context window

The subagent gets a clean context freshly and applies all changes atomically.

## Architectural Decision Gate (NON-NEGOTIABLE)

Before implementing ANY architectural change (protocol critical data flow, cache, transport, API model):
- **ALWAYS ask the user first** (via the `question` tool, with explicit options)
- **NEVER fall back to an alternative architecture** without user approval
- **NEVER silently modify the instruction's setting** (e.g., SFU → P2P) even if it seems "easier"
- **NEVER implement a workaround** that changes the system's design without explicit user consent

If the agreed approach fails, report the failure and ask: "方案 X 失败，原因是 Y。建议改用 Z，是否同意？" <!-- c1:allow-zh -->

## Test Execution Constraint (NON-NEGOTIABLE)

After claiming tests are run by a mock or partially-laid layer:
- **ALWAYS run the tests** against the live system. Writing tests without executing them is a mock.
- **ALWAYS report actual test output** — pass/fail counts, error messages, never claim success without real output.
- If tests fail, fix them in this turn. Do not defer to "in a later turn".

## Verification Honesty (NON-NEGOTIABLE)

- **NEVER claim a feature works based on a partial test.** An example: a Python WS test passing does NOT mean the browser flow works.
- **NEVER report success if you cannot see it yourself** — run the command and read the output.
- Tell plenty about exactly what you tested and NOT what you didn't.
- **If you cannot verify at the user-facing layer, say so explicitly.** Do not imply success.

## Feature Flag Discipline

- **ALL required features MUST be in default features** in Cargo.toml — never require `--features` for core build
- **Build commands in docs MUST include all features** — never document build without the flags
- Default features must build the binary

## Self-Verification Requirement (NON-NEGOTIABLE)

- **Run Playwright MCP tools to verify the page actually renders.** Browsers are real.
- Do not rely on AI relay. Use the actual output tab.

## User Confirmation Before Edit (NON-NEGOTIABLE)

- **NEVER start editing a file without explicit user approval.** Describing a plan is not approval.
- **When the requirement changes mid-way, re-confirm before continuing.**
- **When the requirement changes mid-way, re-confirm before continuing.**
- Silence / timeout ≠ approval. Only explicit "ok / go / continue 执行" counts. <!-- c1:allow-zh -->

## Process Management & Name Safety (shell)

- **NEVER use `pgrep -f` / `pkill -f` with a pattern that matches your own shell cmdline** — the shell kills itself, hanging the tool. Use `pgrep -x <name>` or exclude your own PID.
- **Only reload long-lived processes detached from the tool shell** — use `setsid` / `nohup ... & disown` so the tool is not the supervisor.
- After relaunching a service, confirm it is up with `curl` against its local port; if the port is taken, find the stale process with `ss -tlnp` and kill by PID.
- **curl local services with `--noproxy "*"`** — a shell `http_proxy` makes `curl http://127.0.0.1:PORT` route through the proxy and hang (misread as "server unresponsive").
- **Container tcpdump: mind NAT rewriting** — host→container packets get their source IP rewritten to the gateway; filter by source port to separate local traffic, not by source IP.

**Source**: inherited from MediaServo PIT-54/56/58 (debug rounds: pgrep self-kill, proxy hang, NAT filter).

## Git Recovery Operations

- Batch `git restore` of staged deletions may silently skip paths — `git ls-files` shows the file in the index while it is missing on disk. Use `git checkout HEAD -- <paths>` to force writeback.
- Verify recovery exhaustively, not by sampling: after restoring N directories, loop comparing index file count vs on-disk count per directory.

**Source**: inherited from MediaServo PIT-68.

## Batch `edit` / Patch Safety (enforceable rules)

### 8. Verify line uniqueness after multi-line edits

After a multi-line `replace` on .py/.rs files whose new content contains repeated patterns, `grep -c "<inserted line>" <file>` must return the expected count (usually 1); a higher count means the edit duplicated lines. Prefer python exact-string replacement (read → `assert s.count(old)==1` → replace → write) over blind `edit` retries.

**Blocking condition**: committing a multi-line edit without uniqueness/line-count verification.
**Source**: inherited from MediaServo PIT-78a.

### 9. Grep current state before consecutive edits in the same region

When editing the same function/region of a file repeatedly, first `grep -c "<anchor line>" <file>` to confirm the anchor is unique; for "insert before existing code" patterns prefer python exact replacement over `edit` lines arrays.

**Blocking condition**: a second consecutive edit on the same function without a preceding grep check.
**Source**: inherited from MediaServo PIT-81.

### 10. Python batch-replace scripts: assert each anchor, write per block

In multi-block replacement scripts, every `replace(old, new)` must be preceded by `assert s.count(old) == 1`; write the file back to disk after each block (or pre-validate all asserts first). Never mutate many variables then `write` once at the end — one failed assert mid-script discards all prior work.

**Blocking condition**: single trailing write in a multi-block script, or retrying without checking which blocks already landed.
**Source**: inherited from MediaServo PIT-84.

### 11. Large markdown/memory appends use heredoc, not edit JSON

Appending long markdown sections (with quotes, backticks, tables) via the `edit` tool repeatedly fails JSON parsing. Use `cat >> file <<'EOF'` (or a full-file rewrite), then verify with `grep` for the appended title and `wc -l` for growth.

**Blocking condition**: a failed `edit` append retried instead of switching to heredoc.

### 12. Never run `cargo fmt` workspace-wide for a single file

`cargo fmt -- <path>` is not a path filter — it formats the whole workspace, and version drift between stored files and current rustfmt can produce 100+ file diffs. For one file use `rustfmt --edition <ed> <file>`. After any format op: `git diff --stat | wc -l` must match the expected file count (usually 1).

**Blocking condition**: single-file formatting done via cargo fmt, or diff scope unverified.
**Source**: inherited from MediaServo (2026-08-12 accidental format of 112 files / 3103 insertions).

### 13. On batch-edit hash mismatch, fully re-read before retrying

After a `>>> tag mismatch` error, do not stitch partial LINE#ID tags from the error output — unchanged lines still carry stale tags and the retry fails again. Re-`read` the file (or the full needed window) before the next call. Files that the running agent itself may rewrite (e.g. `~/.config/opencode/*.jsonc`) must always be freshly re-read immediately before editing.

**Blocking condition**: partial-tag retry without re-read; JSON config edited without post-edit syntax validation.

### 14. Repo-wide renames and batch edits are forbidden while subagents run

A background task may commit, overwrite, or `git checkout` the working tree while you hold uncommitted batch changes — your work gets swept. Before any repo-level rename/replace: confirm no background tasks are active. After: verify old patterns are gone (`grep` count = 0) and, for compiled artifacts, do binary-level symbol verification (`readelf`/`nm`).

**Blocking condition**: repo-wide replace executed with unfinished subagents running.
**Source**: inherited from MediaServo PIT-98.

### 15. `pkill` / `pgrep` under `set -e` need `|| true`; timeouts need `-k`

In cleanup code, `pkill <name>` exits 1 when no process matches and kills the script under `set -e`, skipping the rest of the cleanup. Write `pkill <name> || true`. When wrapping long commands with `timeout`, add `timeout -k 5 <t>` so children get a kill signal. For cleanup pkill use `-x <exact-name>`, never `-f`.

**Verification**: `bash -x scripts/<name>.sh` — if the last line shows the pkill followed by trap cleanup, the script fast-exited; diagnose before assuming a hang.
**Blocking condition**: script without `|| true` after pkill/pgrep; timeout without `-k`.
**Source**: inherited from MediaServo PIT-120/121 (cleanup killed by set -e; nohup process uncaptured for 200s).

### 16. `pkill -f` must never match your own cmdline

`pkill -f <string>` where <string> appears in your current shell's command (a path, a binary name you also pass as argument) kills your own shell — the tool call hangs silently. Use `ps -eo pid,comm` and kill by PID, or grep-pattern bracket tricks (`[c]hrome`).

**Blocking condition**: any `pkill -f` / `pgrep -f` pattern with substring overlap with the current cmdline.
**Source**: inherited from MediaServo PIT-170.

### 17. After batch-edit failure, grep state before retry; assert gates capture real exit codes

When a batch `edit` reports a mismatch, some ops may have already applied — re-sending the whole batch duplicates them. Grep/re-read the failed regions first, then apply only the missing ops. Gate checks must not read a pipeline's tail exit code: use `cmd && echo OK` or `set -o pipefail`.

**Blocking condition**: full-batch resend after partial failure; a replace without `assert count==1`; treating a piped command's exit code as the gate result.
**Source**: inherited from MediaServo PIT-177.

### 18. Patch scripts anchor with regex, never fragile literals

In long sessions, literal anchors in python/heredoc patches fail on invisible drift (trailing `2>/dev/null` suffixes, `||` double pipes, backtick placement, frontmatter changes). This cost 4+ failed patch rounds in one session. Write anchors as `re.search(r'^- \*\*Check\*\*: .*marker.*$', s, re.M)` — anchor on stable structure (line start + unique token), then replace the matched span. Assert the regex matched before writing.

**Blocking condition**: two consecutive literal-anchor mismatches in the same batch.
**Source**: this project, 2026-09-28 English-migration batch (4 misses → regex retry pattern).

### 19. Long-session state: grep ground truth before any state-dependent assertion

After 30+ turns, remembered git status / file inventory / directory lists go stale — assertions built on memory misfire (edit hash mismatches from guessed LINE#IDs, "dir X doesn't exist" claims disproven by `ls`). Before any state-dependent operation (patch anchor, git add scope, existence claim), run the cheap ground-truth command first: `git status --short`, `grep -c`, `ls`. Never assert from memory.

**Blocking condition**: an assertion error where the anchor text came from session memory instead of a same-turn read.
**Source**: this project, 2026-09-28 (C1-check anchor missed twice; lang-dir misclaim caught by audit cross-check).

### 20. Revision-line claims must reference a real body change

Writing a plan/doc header line claiming "+X step added" while only the header was patched (body unchanged) creates a phantom feature that a reviewer's grep will catch — cost: one full review lane + a fix round. Same family as PIT-4 (unverified citations). After adding any "revision/N.B. says X was added" line, immediately re-grep the body for the claimed content before moving on.

**Blocking condition**: committing a revision/changelog line whose claimed change is not grep-present in the same diff.
**Source**: this project, 2026-09-28 plan-review (P1-1 phantom schema step + F4, caught independently by 2 lanes).
