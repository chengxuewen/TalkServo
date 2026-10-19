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

- **Constraint**: `docs/` root holds only the whitepaper and the architecture master; **design documents live in `docs/modules/NN-*.md`** (living docs, updated in place as implementation lands, numbered from 01); living reference handbooks under `docs/reference/` (API/config/ops — distinct from design); one-time research archives under `docs/reference/research/<domain>/` (frozen, never back-written; new findings get new dated files). Current domains: `ptt/`, `media/`, `ui/`, `tooling/`. `docs/plans/` is a permitted working tier for openspec proposal artifacts (pre-design drafts). Design ≠ reference ≠ research; no cross-tier mixing.
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
