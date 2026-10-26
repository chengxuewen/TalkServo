---
name: ci-cd-automation
description: "TalkServo local verification pipeline: scripts/gate.sh ledger gates, pixi tasks (build/test/gate/e2e), ctest lanes via the PolyOrch graph, cargo-deny audit. There is no hosted CI (D26) — ci.yml is an inert job spec. Use for local gate troubleshooting, pipeline changes, or pre-push verification."
---

# ci-cd-automation — TalkServo Local Verification Pipeline

> **Ledger note (doc-audit 2026-09-28, PIT-3)**: bare `PIT-{n}` / `C{n}` / `D{nn}` identifiers in this file cite the **MediaServo ledger** (sister project), not this repo — TalkServo's own ids live in `.agents/memorys/` (C1-C6, D1-D28, PIT-1..42).

> **Verification topology (D26, authoritative)**: TalkServo has NO hosted CI. The only remote is Gitee, so `.github/workflows/ci.yml` is an INERT job spec kept for a future GitHub mirror — nothing executes it. Enforcement is local: `.githooks/pre-push` runs `scripts/gate.sh` on every push (enable once per clone: `git config core.hooksPath .githooks`), and the pixi tasks below are the push obligations.

## The Actual Verification Pipeline

Everything routes through the PolyOrch CMake graph (D25/D27/D28): build/test/gate/e2e are ctest labels over `out/cmake-build`. Run tasks through pixi so the toolchain env (meson/clang/Ninja PATH, linker pins) is correct.

```text
pixi tasks (from pixi.toml [feature.dev.tasks] / [feature.ci.tasks])
│
├── cmake-probe   : cmake -B out/cmake-build -G Ninja          # configure/discoverability
├── build         : cmake --build out/cmake-build --target talkservo-lanes-stub-build
├── test          : ctest -L "rust|web"                        # daily lane (5 entries)
├── gate          : ctest -L gate                              # ledger/fmt/clippy/deny
├── e2e           : ctest -L e2e-stub                          # Playwright stub lane
├── test-sfu      : TS_LANE_LIVE=1 ctest -L mediasoup          # live lane (~25 min)
├── lint          : fmt + clippy workspace
├── format        : cargo fmt --all -- --check
├── format-fix    : cargo fmt --all
├── check         : cargo check --workspace
├── audit         : cargo deny check (gate-deny runs it too; needs network for the DB)
├── coverage      : cargo tarpaulin --workspace
├── client-install / web-install / web-build / web-test / web-dev
├── run-server / run-coturn / issue-token
└── ci-check / ci-check-mac / ci-lint / ci-test / ci-test-mediasoup   # [feature.ci.tasks]
```

IDE equivalents (graph buttons, not ALL): `talkservo-gates-build` runs `ctest -L gate`; `talkservo-e2e-stub-build` runs `ctest -L e2e-stub` after the stub aggregate. Full inventory: `docs/modules/09-dev-toolchain.md` §6 (17 targets, D27/D28).

## Pre-Push Verification Sequence

Run in order (the pre-push hook runs the first two automatically; the rest are your push obligations):

```bash
# 0. Ledger + graph gates (the hook runs this): C1-C6, POLYORCH floor, LANES, LINK
bash scripts/gate.sh

# 1. Format + clippy (minutes warm, tens of minutes cold)
~/.pixi/bin/pixi run lint

# 2. Supply-chain audit (needs network for the advisory DB; ~1.4s warm)
~/.pixi/bin/pixi run audit

# 3. Daily graph lane (rust + web + fixture, ~4 min)
~/.pixi/bin/pixi run test

# 4. Full gate incl. deny via the graph
~/.pixi/bin/pixi run gate

# 5. Browser e2e stub lane
~/.pixi/bin/pixi run e2e

# 6. Live SFU lane — ONLY when touching server/media; TS_LANE_LIVE=1 required
TS_LANE_LIVE=1 ~/.pixi/bin/pixi run test-sfu
```

## Quick Reference: Cargo-Direct Tasks

For crate-level iteration outside the graph (note: a manual `cargo …` without `--target-dir` forks whichever cache the lanes are not using — for lane work pass `--target-dir out/cargo-target/{stub,live}`, see modules/09 §6):

| Task | Command | When |
|------|---------|------|
| `pixi run check` | `cargo check --workspace` | After any code change |
| `pixi run lint` | fmt --check + clippy -D warnings | Before commit |
| `pixi run format` / `format-fix` | `cargo fmt --all [-- --check]` | Style pass |
| `pixi run audit` | `cargo deny check` | Pre-push security audit |
| `pixi run coverage` | `cargo tarpaulin --workspace --out Html --out Lcov` | Coverage report |

There is deliberately NO `check-fast`/`build-fast`/`test-server`/`test-gstreamer` task — the fast path is `pixi run check` (stub features need no C++ worker for compile checks) and the server test path is the graph lane `TS_LANE_LIVE=1 pixi run test-sfu` or the stub lane inside `pixi run test`.

## mediasoup / Linux-Only Reality

- mediasoup's C++ Worker builds on Linux x86_64 only. On macOS: `cargo check` works; `build`/`test` with `sfu-mediasoup` do not (use a Linux box or the hand-run Docker parity image).
- The live lane (`TS_LANE_LIVE=1 pixi run test-sfu`) compiles the worker: measured ~25 min warm-cache, and a killed mid-download build can poison the meson subproject cache (PIT-39) — never SIGKILL a downloading build.
- Host toolchain comes from pixi (`pixi run -e dev`); a bare shell cmake/ninja is not the supported entry.

## cargo-deny Audit

```bash
~/.pixi/bin/pixi run audit   # cargo deny check
```

| Check | Config | Threshold |
|-------|--------|-----------|
| Security advisories | `deny.toml [advisories]` | Deny yanked crates |
| License compliance | `deny.toml [licenses]` | Allow-list per deny.toml |
| Duplicate deps | `deny.toml [bans]` | Warn on multiple versions |
| Source registry | `deny.toml [sources]` | Deny unknown registries/git |

A red `gate-deny`/`audit` means check connectivity first — the advisory DB fetch needs the network and fails transiently through some proxies (measured 2026-10-21/22).

## Common Pitfalls

| Pitfall | Reference | Fix |
|---------|-----------|-----|
| MESON must be absolute path | MediaServo PIT-12 | env comes from pixi activation; never set it by hand |
| cargo clean -p doesn't clear build cache | MediaServo PIT-13 | `rm -rf target/debug/build/<pkg>-*` |
| Shared cargo target dir refuses `cargo clean -p` | TalkServo PIT-41 | deletion is per cache group (`talkservo-lanes-{stub,live}-clean`) |
| Phantom cargo failure during a parallel cargo | TalkServo rule (edit-safety 22) | check `pgrep -x cargo` empty; rerun once before diagnosing |
| gate-clippy claims green but ran fmt | TalkServo PIT (c1334f9) | configure-time assertion in TalkServoGates.cmake guards the argv |

## Related Skills

| Skill | Relationship |
|-------|-------------|
| `review-hardcode` | Run BEFORE audit — catch hardcoded secrets in CI config |
| `test-harness` | Generate test skeletons that the lanes will run |
| `lesson-memory` rule | Build/gate failures → write to pitfalls.md |
| `think-before-act` | Reproduce a gate failure locally BEFORE implementing a fix |
