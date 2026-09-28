# PROJECT KNOWLEDGE BASE

**Generated:** 2026-09-28 | **Commit:** 34300f1 | **Branch:** main

## OVERVIEW

TalkServo — real-time voice **floor-control platform** (PTT exclusive / conference open / hybrid), pre-implementation: design-doc-driven PoC. Planned stack: Rust core/server + mediasoup SFU (D6) + React/TS web SPA (D9) + Electron desktop shell (D10). Sister project of MediaServo (same RTC domain; conventions shared, ledgers NOT).

## STRUCTURE

```
.
├── .agents/        # AI infrastructure: rules/ skills/ memorys/ (ledgers) — see its AGENTS.md
├── .opencode/      # opencode config: opencode.json (instructions injection list), init-mcp-*.mjs launchers
├── .omo/           # agent runtime state (run-continuation/ is git-ignored; only omo.jsonc tracked)
├── docs/           # whitepaper + architecture MASTER + modules/ (design) + reference/ (living + frozen research) — see its AGENTS.md
├── crates/         # DOES NOT EXIST YET — workspace = plan step P0-1 (layout: docs/modules/03)
├── web/            # DOES NOT EXIST YET — PoC SPA (docs/modules/08)
├── scripts/ config/ docker/ pixi.toml  # DO NOT EXIST YET — land with P0-1 (docs/modules/09)
└── package.json    # only real code today: codegraph MCP dev-dep
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| What are we building / why | docs/whitepaper.md | v0.1 planning voice |
| Architecture truth (live) | docs/architecture.md → modules/01-09 | master is thin by design |
| Why a decision exists | .agents/memorys/decisions.md D1-D11 | cite by id |
| Binding rules on this repo | .agents/memorys/conventions.md C1-C2 | C1 English-artifacts, C2 docs-tiering |
| Engineering lessons (ours) | .agents/memorys/pitfalls.md PIT-1..3 | MediaServo ids labeled, never assume |
| Competitive/selection evidence | docs/reference/research/{ptt,media,ui,tooling}/ | FROZEN dossiers |
| Next implementation step | .agents/memorys/status.md → Next Steps | P0-1 workspace slice |
| SDK/binding future shape | docs/modules/07 + D8 | nothing built at Beta trigger, not before |

## CODE MAP

Unmeasured — zero source files; no LSP/codegraph index applicable pre-code. Symbol inventory starts when `crates/talkservo-core` lands (seams to map: `apply()`, `Sfu` trait, `SignalingMessage`).

## CONVENTIONS (deviations only — full text lives in ledgers, do not restate here)

- **C1**: every artifact English; only AI↔user chat follows user language. Functional Chinese literals need inline `c1:allow-zh`.
- **C2**: docs tiering — design→`docs/modules/NN`, knowledge→`docs/reference/`, frozen research→`reference/research/<domain>/`.
- **Ledger-first citation**: any C/D/PIT id in any file must resolve against `.agents/memorys/` or carry a "MediaServo" label (PIT-3).
- **Cargo.lock committed**; mediasoup UDP/ICE constraints in `.agents/rules/common/{docker,platform}.md` (Linux-native-first per D11 — their macOS posture is conditional).

## ANTI-PATTERNS (THIS PROJECT — learned in-session, enforced)

- **Never trust agent self-reports** on batch file writes: repo-grep counts are the only evidence (PIT-1).
- **Never rebase --root to rewrite history** when ignored runtime files block checkout — use `commit-tree` + `update-ref` (PIT-2).
- **Never port prose without a referential-integrity join** (ids/paths/ports → target ledger) (PIT-3).
- **No speculative scaffolding**: empty crate dirs, hardware-capacity docs, "for later" bindings — content earns its directory (modules/03 P0-1, gap audit refusal).
- **No back-writing frozen dossiers**; superseding decisions annotate the live doc, research stays as evidence.
- **`openspec-explore`/`openspec-propose` skills contain DeskServo-lineage architecture illustrations** — flagged porting warnings; trust docs/modules/, not their examples (adjudication pending).

## COMMANDS

```bash
# gates (run before claiming doc/config work complete)
grep -rnP '[\x{4e00}-\x{9fff}]' --exclude-dir=node_modules --exclude-dir=.git \
  .agents .opencode .omo docs web 2>/dev/null | grep -v c1:allow-zh        # C1: must be empty
test -f docs/reference/README.md && test -d docs/modules && ! ls docs/research 2>/dev/null && echo C2-OK
python3 - <<'E'  # docs link integrity
import re,os,glob
print(sum(not os.path.exists(os.path.normpath(os.path.join(os.path.dirname(p),m)))
  for p in glob.glob("docs/**/*.md",recursive=True)
  for m in re.findall(r'\]\(([^)h][^)]*\.md)',open(p).read()))) # must print 0
E
# future (P0-1): pixi run test / lint / test-sfu — see docs/modules/09
```

## NOTES

- Working tree ahead of last commit (post-audit fixes uncommitted at generation time).
- Name availability: crates.io `talkservo` **taken** (placeholder, 0 versions) — reservation strategy before publishing (research/github-sweep-ptt).
- Dev host is Linux x86_64 → mediasoup builds native; Docker = CI-parity only (D11).
- `.omo/run-continuation/` files churn constantly — never stage them.
