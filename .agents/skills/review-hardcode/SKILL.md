---
name: review-hardcode
description: "Hardcoded secrets/ports/URLs scan. MERGED into security-hardening — loads that skill's Phase 1. Use /review-hardcode for quick scan, /security-hardening for full audit."
---

# review-hardcode → security-hardening

> **Ledger note (doc-audit 2026-09-28, PIT-3)**: bare `PIT-{n}` / `C{n}` / `D{nn}` identifiers in this file cite the **MediaServo ledger** (sister project), not this repo — TalkServo's own ids live in `.agents/memorys/` (C1-C2, D1-D11, PIT-1-PIT-3).


This skill has been merged into the `security-hardening` skill.

**`/review-hardcode`** now loads Phase 1 of `security-hardening`: secret scan (including the hardcoded value severity table, MediaServo PIT-10 rule, and scan-hardcode.sh).

For a full security audit, use `/security-hardening`.
