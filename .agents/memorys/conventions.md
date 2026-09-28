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
- **Check**: `grep -rnP '[\x{4e00}-\x{9fff}]' --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=target --exclude-dir=__pycache__ .agents .opencode .omo crates admin-dashboard docs web 2>/dev/null | grep -v 'c1:allow-zh'` (output must be empty; non-existent dirs skipped by 2>/dev/null)
- **Source**: User instruction (2026-09-28 documentation-port adaptation round; 2026-09-28 tightening round)

<!-- Subsequent conventions start here -->

## C2: docs tree uses Diataxis tiering `reference/` + `reference/research/<domain>/` (2026-09-28)

- **Constraint**: `docs/` root holds only the whitepaper and the architecture master; **design documents live in `docs/modules/NN-*.md`** (living docs, updated in place as implementation lands, numbered from 01); living reference handbooks under `docs/reference/` (API/config/ops — distinct from design); one-time research archives under `docs/reference/research/<domain>/` (frozen, never back-written; new findings get new dated files). Current domains: `ptt/`, `media/`, `ui/`, `tooling/`. `docs/plans/` is a permitted working tier for openspec proposal artifacts (pre-design drafts). Design ≠ reference ≠ research; no cross-tier mixing.
- **Rationale**: aligns with sister projects DeskServo/MediaServo layout (user ruling 2026-09-28); separating snapshot research from live spec keeps stale conclusions out of the living surface.
- **Check**: `test -f docs/reference/README.md && test -d docs/modules && ! ls docs/research 2>/dev/null && echo OK` (must print OK; `docs/modules/` must not hold external research; `reference/research/` must not hold design; links from `docs/reference/research/*/*.md` to docs root must use the `../../../` prefix)
- **Source**: User instruction (research-docs structure round, 2026-09-28)
