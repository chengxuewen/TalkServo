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
├── crates/         # workspace: talkservo-core (39 tests) / -sfu / -server (79) — plan-1..6 EXECUTED
├── web/            # SPA (Vite+React18+AntD5): dispatch/field/participant/admin pages + e2e
├── packages/client # TS SDK (22 tests, unpublished D14)
├── scripts/ config/ docker/ pixi.toml  # toolchain + coturn + lanes (docs/modules/09)
└── package.json    # codegraph MCP dev-dep
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| What are we building / why | docs/whitepaper.md | v0.1 planning voice |
| Architecture truth (live) | docs/architecture.md → modules/01-09 | master is thin by design |
| Why a decision exists | .agents/memorys/decisions.md D1-D25 | cite by id; D23 = Alpha wave-1 (plan-7, S1-S7.5 landed), D25 = PolyOrch build graph |
| Binding rules on this repo | .agents/memorys/conventions.md C1-C3 | C1 English, C2 docs-tiering, C3 full-word routes |
| Engineering lessons (ours) | .agents/memorys/pitfalls.md PIT-1..25 | MediaServo ids labeled, never assume |
| Competitive/selection evidence | docs/reference/research/{ptt,media,ui,tooling}/ | FROZEN dossiers |
| Next implementation step | .agents/memorys/status.md → Next Steps | P0-1 workspace slice |
| SDK/binding future shape | docs/modules/07 + D8 | nothing built at Beta trigger, not before |

## CODE MAP

Measured 2026-10-19: core = `FloorState::apply()` rules 1-9 (crates/talkservo-core/src/floor.rs, 39 tests incl. PIT-21 rebroadcast); server = WS signaling (ws_flow), admin REST (admin.rs), accounts/kdf (D22), lifecycle/instance (D19-D21); web = session bridge (lib/session.ts) + 4 pages; SDK = TalkServoClient + MediaManager + MediasoupClientStack (packages/client). Codegraph index not yet built — use Grep/Glob.

## CONVENTIONS (deviations only — full text lives in ledgers, do not restate here)

- **C1**: every artifact English; only AI↔user chat follows user language. Functional Chinese literals need inline `c1:allow-zh`.
- **C2**: docs tiering — design→`docs/modules/NN`, knowledge→`docs/reference/`, frozen research→`reference/research/<domain>/`.
- **C3**: user-facing route/CLI/API names use full mainstream words (`/admin`, `/field/:room`, `/dispatch/:room`, `/room/:room`) — no cryptic abbreviations.
- **Ledger-first citation**: any C/D/PIT id in any file must resolve against `.agents/memorys/` or carry a "MediaServo" label (PIT-3). Ranges: D1-D25, C1-C3, PIT-1..25.
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
grep -rnP '[\x{4e00}-\x{9fff}]' --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=dist --exclude-dir=coverage \
  .agents .opencode .omo docs web 2>/dev/null | grep -v c1:allow-zh        # C1: must be empty
test -f docs/reference/README.md && test -d docs/modules && ! ls docs/research 2>/dev/null && echo C2-OK
python3 - <<'E'  # docs link integrity
import re,os,glob
print(sum(not os.path.exists(os.path.normpath(os.path.join(os.path.dirname(p),m)))
  for p in glob.glob("docs/**/*.md",recursive=True)
  for m in re.findall(r'\]\(([^)h][^)]*\.md)',open(p).read()))) # must print 0
E
~/.pixi/bin/pixi run test    # graph: ctest -L rust|web (P2 forwarder; direct parity = cargo test --workspace)
~/.pixi/bin/pixi run gate && ~/.pixi/bin/pixi run e2e    # P3: ledger gates + graph e2e (full: ctest -L "rust|web|gate|e2e-stub")
~/.pixi/bin/pixi run cmake-probe && ~/.pixi/bin/pixi run ctest --test-dir out/cmake-build -L rust   # plan-8 graph (P1: core lane)
~/.pixi/bin/pixi run lint    # fmt+clippy
cd web && npx vitest run && npx tsc --noEmit   # web unit + types
cd web && npx playwright test e2e/floor.spec.ts e2e/talk.spec.ts   # stub-lane e2e
```

## NOTES

- Doc-audit 2026-10-19 (D24): forward pointers in modules/01/08, architecture §4, server-api mark adjudicated wave-1 scope; full rewrites land with plan-7 §8 sync obligations.
- Name availability: crates.io `talkservo` **taken** (placeholder, 0 versions) — reservation strategy before publishing (research/github-sweep-ptt).
- Dev host is Linux x86_64 → mediasoup builds native; Docker = CI-parity only (D11).
- `.omo/run-continuation/` files churn constantly — never stage them.
