# TalkServo Conventions & Constraints

> This file records development constraints accumulated by TalkServo itself. Format: `## C{n}: Title (date)` — "constraint" + rationale + check command + source.
> Numbering starts at C1 and increases consecutively; every entry must be executable (have a check command) — no empty "be careful with XXX" wording.
>
> Note: when the documentation system was ported from MediaServo on 2026-09-28, its C1-C46 ledger was cleared as part of the project reset.
> If a MediaServo convention of the same kind (mediasoup boundary, iceoryx2 cleanup, libwebrtc timestamp discipline, etc.) is actually
> hit/used in this project, re-file it in its format and mark it "inherited from MediaServo C{n}" — never pre-copy unverified constraints.

## C1: All project artifacts are in English; only AI-agent interaction follows the user's language (2026-09-28)

- **Constraint**: All project artifacts — source code, identifiers, comments, doc-comments, commit messages, PR descriptions, docs/ documents, configuration file text, CI output, and the `.agents/memorys/` ledgers — are written in English. The only exception: the AI agent's interaction surface with the user (replies, questions, reports) follows the user's input language — if the user writes in Chinese, the agent replies in Chinese. Functional Chinese match literals (user trigger words, regexes grepping Chinese content) are allowed, but each such line must carry the inline marker `c1:allow-zh`.
- **Rationale**: User ruling 2026-09-28, tightened the same day (memorys exemption removed). Code assets target international collaboration; agent interaction prioritizes user efficiency.
- **Check**: `grep -rnP '[\x{4e00}-\x{9fff}]' --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=target --exclude-dir=__pycache__ --exclude-dir=dist --exclude-dir=coverage .agents .opencode .omo crates admin-dashboard docs web 2>/dev/null | grep -v 'c1:allow-zh'` (output must be empty; build artifacts carry vendored locales — AntD ships Chinese strings in `web/dist`, so dist/coverage are excluded; non-existent dirs skipped by 2>/dev/null)
- **Source**: User instruction (2026-09-28 documentation-port adaptation round; 2026-09-28 tightening round)

```bash
out=$(grep -rnP '[\x{4e00}-\x{9fff}]' --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=target \
  --exclude-dir=__pycache__ --exclude-dir=dist --exclude-dir=coverage \
  .agents .opencode .omo crates admin-dashboard docs web 2>/dev/null | grep -v 'c1:allow-zh')
if [ -n "$out" ]; then echo "FAIL: non-English artifacts:"; echo "$out"; exit 1; fi
echo PASS
```

<!-- Subsequent conventions start here -->

## C2: docs tree uses Diataxis tiering `reference/` + `reference/research/<domain>/` (2026-09-28)

- **Constraint**: `docs/` root holds only the whitepaper and the architecture master; **design documents live in `docs/modules/NN-*.md`** (living docs, updated in place as implementation lands, numbered from 01); living reference handbooks under `docs/reference/` (API/config/ops — distinct from design); one-time research archives under `docs/reference/research/<domain>/` (frozen, never back-written; new findings get new dated files). Current domains: `ptt/`, `media/`, `ui/`, `internal/` (audit/archive dossiers). `docs/plans/` is a permitted working tier for openspec proposal artifacts (pre-design drafts). Design ≠ reference ≠ research; no cross-tier mixing.
- **Rationale**: aligns with sister projects DeskServo/MediaServo layout (user ruling 2026-09-28); separating snapshot research from live spec keeps stale conclusions out of the living surface.
- **Check**: `test -f docs/reference/README.md && test -d docs/modules && ! ls docs/research 2>/dev/null && echo OK` (must print OK; `docs/modules/` must not hold external research; `reference/research/` must not hold design; links from `docs/reference/research/*/*.md` to docs root must use the `../../../` prefix)
- **Source**: User instruction (research-docs structure round, 2026-09-28)

```bash
test -f docs/reference/README.md && test -d docs/modules && ! ls docs/research 2>/dev/null && echo PASS
```

## C3: User-facing route/CLI/API names use full mainstream words — no cryptic single-letter abbreviations (2026-10-09)

- **Constraint**: Every externally-visible name — SPA route paths, CLI subcommands, REST endpoints, generated link URLs — uses full, self-explanatory English words aligned with mainstream convention (`/admin`, `/field/:room`, `/dispatch/:room`, mirroring `/admin/api/*`). Single-letter or in-house-abbrev path segments are forbidden unless they ARE the industry term (e.g. `/ws`). When naming such a surface, cite one mainstream precedent in the design doc line.
- **Rationale**: User ruling 2026-10-09 after `/a`+`/f`+`/d` were questioned as "abnormal paths" — the originals were arbitrary first-thought choices, self-inconsistent with the `/admin/api` REST surface in the same system; join links and console URLs are user-visible contracts.
- **Check**: `grep -rnE 'path="/(a|d|f)(/|")|"/(a|d|f)/"' web/src web/e2e crates/talkservo-server/src 2>/dev/null` must be empty (the 2026-10-09 rename missed `web/e2e/floor.spec.ts` URL builders — the gate must cover e2e too); route table in docs/modules/08 lists only full-word paths.
- **Source**: Route rename batch 2026-10-09 (web/src/main.tsx, admin.rs URL templates, e2e specs, modules/08); user directive "主流使用 admin 这样的路径…并记忆记录避免以后再犯". <!-- c1:allow-zh -->

```bash
out=$(grep -rnE 'path="/(a|d|f)(/|")|"/(a|d|f)/"' web/src web/e2e crates/talkservo-server/src 2>/dev/null)
if [ -n "$out" ]; then echo "FAIL: single-letter route paths:"; echo "$out"; exit 1; fi
echo PASS
```

## C4: ledger *indexes* must be regenerated-consistent with the ledgers they describe (2026-10-21)

- **Constraint**: any file that summarizes ledger contents rather than defining them — `AGENTS.md` (the `Ranges:` row and the WHERE-TO-LOOK `D1-Dn` / `C1-Cn` / `PIT-1..n` rows) and `.agents/memorys/status.md` (the Decision Status table) — must state exactly what the ledgers hold: the D/C/PIT upper bounds equal the highest actual id, and the status.md table carries one row per `## D{n}` entry, with no id missing and no row citing an id that has no ledger entry. Adding a ledger entry updates its index rows **in the same commit**.
- **Rationale**: two drifts inside one session (2026-10-21): `AGENTS.md` still advertised `PIT-1..24` after PIT-25/26 existed, and the status.md Decision Status table skipped D23/D24 entirely (`D22 → D25`) while the same repo cites "every C/D/PIT id must resolve" one section later. A stale index is a broken foreign key — and the index is the file a newcomer reads first, so the drift is invisible exactly where it hurts most. C3's own "cite one precedent" discipline deserves the same mechanical backing.
- **Check**: `bash scripts/gate.sh` runs the C4 block (embedded verbatim below, refreshed by `python3 scripts/gate-gen.py`); it fails on a stale range in either AGENTS.md row, on a decision id missing from the status table, and on a table row with no ledger entry behind it. Verified against three mutations (range rolled back / row deleted / row invented) before being adopted.
- **Source**: this project, session 2026-10-21 (plan-7 S7.5 + S7.4 close; see the S7.4 close note and PIT-27).

```bash
set -uo pipefail
dmax=$(grep -o '^## D[0-9]*' .agents/memorys/decisions.md | grep -o '[0-9]*' | sort -n | tail -1)
pmax=$(grep -o '^## PIT-[0-9]*' .agents/memorys/pitfalls.md | grep -o '[0-9]*' | sort -n | tail -1)
cmax=$(grep -o '^## C[0-9]*' .agents/memorys/conventions.md | grep -o '[0-9]*' | sort -n | tail -1)
ids=$(grep -o '^## D[0-9]*' .agents/memorys/decisions.md | sed 's/^## //' | sort)
rows=$(grep -oE '^\| D[0-9]+ \|' .agents/memorys/status.md | tr -d '| ' | sort)
fail=0
grep -q "Ranges: D1-D$dmax, C1-C$cmax, PIT-1..$pmax\." AGENTS.md \
  || { echo "FAIL: AGENTS.md Ranges row stale (want D1-D$dmax, C1-C$cmax, PIT-1..$pmax)"; fail=1; }
grep -q "pitfalls.md PIT-1..$pmax " AGENTS.md \
  || { echo "FAIL: AGENTS.md WHERE-TO-LOOK PIT range stale (want PIT-1..$pmax)"; fail=1; }
missing=$(comm -23 <(echo "$ids") <(echo "$rows"))
[ -n "$missing" ] && { echo "FAIL: status.md Decision Status table missing ids: $(echo $missing | tr '\n' ' ')"; fail=1; }
ghosts=$(comm -13 <(echo "$ids") <(echo "$rows"))
[ -n "$ghosts" ] && { echo "FAIL: status.md table cites ids with no ledger entry: $(echo $ghosts | tr '\n' ' ')"; fail=1; }
[ "$fail" -eq 0 ] && echo PASS
```

## C5: living docs may not cite hosted/automatic verification as if it executes — name the runnable command or mark it inert (2026-10-22)

- **Constraint**: in living docs (`AGENTS.md`, `docs/architecture.md`, `docs/README.md`, `docs/modules/`, the living `docs/reference/*.md`), any sentence about CI, nightly runs, hosted runners, "CI check target" or "CI-parity" must either (a) name the command a human actually runs, or (b) carry an explicit `inert` / `D26` qualifier on the same line. Frozen dossiers under `docs/reference/research/` are exempt — they are never back-written (PIT-3), which is why the check is path-scoped instead of repo-wide.
- **Rationale**: D26 found five jobs in `.github/workflows/ci.yml` that had never executed *and* six living-doc lines presenting a dual-OS CI, a `nightly e2e` and a "CI check target" as if some machine honored them. A doc that implies "CI checks this" outsources the check to nobody; the belief is exactly what lets a lane rot unexamined — the same failure family as an env-gated spec that self-skips green (PIT-27) and an assertion that passes for the wrong reason (PIT-33). C4 keeps the *ledger indexes* honest; C5 keeps the *verification claims* honest.
- **Check**: embedded verbatim in `scripts/gate.sh` (refresh with `python3 scripts/gate-gen.py`, never hand-edit the embedded region). Coverage includes living docs AND the injected `.agents/` surface: skills (`*/SKILL.md`), rules (`**/*.md`), and `memorys/{status,decisions}.md` — a rule file opencode injects every turn is exactly where a phantom-CI sentence does damage.
- **Source**: this project, session 2026-10-22 (CI-location ruling = option C / D26; the drift sweep that found the five lines).

```bash
out=$(grep -rnE 'GitHub Actions|actions/runs|nightly|CI check target|CI lane|CI[- ]parity' \
  AGENTS.md docs/architecture.md docs/README.md docs/modules \
  docs/reference/server-api.md docs/reference/admin-events.md docs/reference/README.md \
  .agents/skills .agents/rules .agents/memorys/status.md .agents/memorys/decisions.md 2>/dev/null \
  | grep -viE 'inert|D26')
if [ -n "$out" ]; then echo "FAIL: hosted-verification claims with no runnable command and no inert marker:"; echo "$out"; exit 1; fi
echo PASS
```

## C6: ledger ids cited anywhere must resolve locally or carry a MediaServo/sister label (2026-10-23)

- **Constraint**: any `D{n}` / `C{n}` / `PIT-{n}` id cited in living files — docs (`docs/**.md` excluding `reference/research/`), `.agents/rules/**`, `.agents/skills/*/SKILL.md`, `.agents/memorys/{status,decisions}.md`, and `.rs` comments under `crates/` — must either resolve against `.agents/memorys/` (within the AGENTS.md ranges) or carry an explicit `MediaServo` / `sister` marker on the same line. Unlabeled foreign ids fabricate project history (the PIT-3 trap) and mislead every later agent.
- **Rationale**: full-audit 2026-10-23 found eight unresolvable cites (`D202`, `C13`, `C9`, `D196`, `PIT-160`, `D39`/`D43`/`D66` in foreign contexts) across injected rules, skills, living docs and code comments. Local-lens citation discipline already exists in prose; it was not mechanical.
- **Check**: embedded verbatim in `scripts/gate.sh` (refresh with `python3 scripts/gate-gen.py`). Scans for `\bD[0-9]+\b`, `\bC[0-9]+\b`, `\bPIT-[0-9]+\b` in the covered files; an id is a FAIL unless it resolves within ranges DERIVED FROM the ledgers themselves (the C4 method — adding a PIT needs no gate edit) or the line matches `MediaServo|sister|inert`. Exemptions: `docs/reference/research/` (frozen), `docs/plans/` (historical working tier), this conventions file itself, and generic tokens like `CMAKE` (word-boundary + no-letter-prefix rules keep false positives near zero).
- **Source**: full doc-audit 2026-10-23, user adjudication #9 ("标来源+加 gate"). c1:allow-zh

```bash
python3 - <<'PY'
import re, sys
from pathlib import Path

def _max_id(path, pat):
    try:
        ids = re.findall(pat, open(path, encoding='utf-8').read(), re.M)
    except OSError:
        return 0
    return max((int(x) for x in ids), default=0)

DMAX = _max_id('.agents/memorys/decisions.md', r'^## D(\d+)')      # live bounds from the ledgers
CMAX = _max_id('.agents/memorys/conventions.md', r'^## C(\d+)')   # (C4 method: derived,
PMAX = _max_id('.agents/memorys/pitfalls.md', r'^## PIT-(\d+)')  #  never re-pinned per entry)
paths = []
for pat in ["docs/**/*.md", ".agents/rules/**/*.md", ".agents/skills/*/SKILL.md"]:
    paths += [p for p in Path('.').glob(pat)
              if 'reference/research' not in str(p) and 'docs/plans' not in str(p)]
paths += [Path('.agents/memorys/status.md'), Path('.agents/memorys/decisions.md')]
paths += [Path('.agents/AGENTS.md')]
paths += [p for p in Path('crates').rglob('*.rs')]

bad = []
for p in paths:
    for i, line in enumerate(p.read_text().splitlines(), 1):
        ids = re.findall(r'\bD\d+\b|\bC\d+\b|\bPIT-\d+\b', line)
        keep = []
        for tok in ids:
            if tok.startswith('PIT-'):
                n = int(tok[4:])
                if not (1 <= n <= PMAX): keep.append(tok); continue
            else:
                n = int(tok[1:])
                if tok.startswith('D') and not (1 <= n <= DMAX): keep.append(tok); continue
                if tok.startswith('C') and not (1 <= n <= CMAX): keep.append(tok); continue
        if keep and not re.search(r'MediaServo|sister|inert', line):
            bad.append(f'{p}:{i}: {keep} {line.strip()[:100]}')
for b in bad: print('FAIL:', b)
raise SystemExit(1 if bad else 0)
PY
```
