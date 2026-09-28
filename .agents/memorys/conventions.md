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
- **Check**: `grep -rPn '[\x{4e00}-\x{9fff}]' .agents .opencode .omo crates admin-dashboard docs 2>/dev/null | grep -v 'c1:allow-zh'` (output must be empty; directories not yet created are skipped by 2>/dev/null)
- **Source**: User instruction (2026-09-28 documentation-port adaptation round; 2026-09-28 tightening round)

<!-- Subsequent conventions start here -->
