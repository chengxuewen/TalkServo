# Plan-6: Ship tooling — oxmgr instance model + talk servo.sh (build/start/deploy/package)

> Sister-project survey (ms_rtc: msrtc.sh → mediaservo.sh → mediaservo_cli.py + oxmgr) drove this design.
> **STATUS: COMPLETE 2026-10-08** (T1 a/b/c, T2, T3, T4; live smokes: facade lifecycle on real oxmgr + package→deploy→start→healthz→stop). Adjudication record at bottom (5 points, user-adjudicated one-by-one 2026-10-08).
> Design truth after execution: `docs/modules/09-dev-toolchain.md` §Delivery shell (pre-staged below) + `docs/modules/06-deployment-security.md` config precedence.

## Command surface (as adjudicated)

Outer shell `talkservo.sh` (repo root, msrtc.sh lineage — pixi activation, no CLI re-implementation):

| Command | Behavior |
|---|---|
| `build [--lane stub\|live] [--debug]` | cargo build the lane feature combo (live = sfu-mediasoup+embedded-web; stub = stub-media tests/tooling host); stage binary into `out/<lane>/bin/`. Stateful-free: never deploys. |
| `build:deploy [--lane ...] [--prefix ...]` | one-step build→deploy composite (sister D266 shape), args pass through to both |
| `package [--lane live\|stub]` | staging → `dist/talkservo-server-<ver>.tar.gz`, versioned top dir (bare extract never scatters), `tar xzf -C <prefix> --strip-components=1` documented; contents: bin + oxmgr bin + etc template + sample.env + systemd unit + Caddyfile.lan + scripts/ |
| `deploy [--prefix DIR] [--lane]` | stateful assembly ONLY (never builds); **default prefix `out/<lane>`** (delivery tree doubles as instance — sister convention; same-path short-circuit); explicit DIR installs elsewhere. Idempotent instance dir `DIR/{bin,etc/server.yaml,identity/ secrets 0600,run/oxfile.toml,logs}`; /opt needs root — check and say so |
| `up / down [--env dev\|prod]` | compose pass-through (server+coturn; env file wiring, `--announced-ip` passthrough to TS_ANNOUNCED_IP/TS_PUBLIC_BASE) |
| `run [--lane stub\|live]` | dev foreground: env assembly + secret hints (today's `pixi run run-server` superset) |
| `doctor` | pixi/cargo/node/meson/clang/docker/oxmgr/ports presence; exit code = failure count |
| `build:client` / `package:sdk` / `build:desktop` | **exit 2 + pointer** to D15/D8/D10 (adjudicated #5A — no scaffold crates) |

Instance lifecycle inside the server binary (facade subcommands, oxmgr does the supervising — adjudicated #1B/#2A):

| Subcommand | Behavior |
|---|---|
| `talkservo-server init <dir>` | generate `etc/server.yaml` (templated config, all current keys) + `identity/` TS_JWT_SECRET/TS_ADMIN_SECRET/TURN_SECRET bootstrap idempotent 0600 + `run/oxfile.toml` (process declaration: exec bin, env_file, logs paths) |
| `start/stop/restart <dir>` | oxmgr apply / stop+delete / stop+apply (fall back with clear error when oxmgr absent from PATH — never silently) |
| `status <dir>` | oxmgr process table + `/healthz` probe column; exit 0 healthy / 1 degraded / 2 not-running |
| `logs / monit / ps <dir>` | oxmgr forwarding (monit = its TUI: procs/CPU/RAM/log stream) |
| `run` (or no subcommand) | today's guarded direct start — systemd/compose/existing scripts zero break |

## Slices (each: implement → verify → commit)

### T1 server: YAML config layer + init/facade
- serde YAML dependency selection via source-driven verification (serde_yaml is EOL — candidates serde_yml / yaml-rust2; pick maintained + workspace-compatible, lock in Cargo.toml + deny.toml review)
- `Config::load(path) -> merged`: precedence **process env > server.yaml > defaults** (documented; env keeps systemd/compose injection authoritative)
- subcommand dispatch in main.rs (run as fallback = zero breakage); init templates as pure `#[cfg(test)]`-covered renderers (MediaServo templates.rs discipline: zero process side-effects in render functions)
- gates: unit (merge precedence, idempotent secrets, template render), stub workspace green, lint clean

### T2 oxmgr archive + injection
- `3rdparty/oxmgr/` upstream musl tar.gz + .sha256 (x86_64; aarch64 optional — record if downloaded); README = provenance/consumption/bump recipe (sister scheme-Y format)
- talk servo.sh arch-case → unpack to `.cache/oxmgr/<archive-name>/` (gitignored, version-in-dir = bump invalidates naturally) → sha256 verify BEFORE unpack, mismatch = refuse loudly → PATH inject; unknown arch = explicit WARN, never wrong-arch binary
- smoke: `start` on a temp instance → `status` exit 0 → `stop` (stub lane, ephemeral ports)

### T3 talk servo.sh shell + package/deploy
- dispatch + env activation (pixi detect same shape as mediaservo.sh; -h skips activation)
- build lanes → out/; package staging → versioned tar; deploy assembly (idempotent, re-deploy keeps identity/ untouched); `doctor`
- tests: `scripts/tests/ship_smoke.sh` — build:stub → package → extract into tmp prefix → init → start/stop via installed bin (lane-stub, no docker), assert tar top-dir + strip-components contract holds

### T4 docs close-out
- modules/09 §Delivery shell → status landed; modules/06 config table gains precedence line; docs/README plan row; AGENTS commands pointer to talk servo.sh; status.md phase line

## Global constraints
- NO wire/signaling/media-plane change; systemd unit + compose stay authoritative prod paths (facade serves operator/dev/edge-box ergonomics)
- talk servo.sh is bash, no Python CLI layer (3-crate PoC ceiling); server keeps its own facade because oxmgr needs a declarator
- oxmgr pinned to one upstream release (0.5.0); bump recipe = archive swap + sha256 re-verify (no silent updates)
- C1 English artifacts; every slice gated; version single-source = workspace Cargo.toml (`_crate_version` equivalent via cargo metadata)

## Adjudication record (user-adjudicated one-by-one, 2026-10-08)

| # | question | decision |
|---|---|---|
| 1 | lifecycle owner | B — facade in binary (no daemonizer re-invention; TUI constraint noted) |
| 2 | mgr carrier | A — adopt oxmgr (scheme-Y archive) + server facade; monit TUI; business view stays web /a |
| 3 | remote deploy | B — local --prefix + tar carry; --remote skin deferred, no rework |
| 4 | config shape | A — YAML instance config (user: readability beats TOML); server gains loader; **env > yaml**; YAML dep source-verified at T1 |
| 5 | absent-artifact slots | A — exit 2 + pointer (D8/D10/D15); no scaffold crates (anti-pattern honored) |

Open items: none (entry named talk servo.sh by sister convention — override at will).

## Post-closeout revisions (2026-10-08)
- deploy default `--prefix out/<lane>` (sister parity; same-path readlink short-circuit) and `build:deploy` composite — user-requested, both smoke-verified (default tree staged-in-place + explicit-DIR copy: init → start → status healthy → stop). Known limit logged: two default-bind instances collide on :8080 (no interactive takeover yet; diverge via identity/env).

## D20 addendum (2026-10-08, user: sister parity for cluster shape)
Default `init` = **server + caddy TLS front** two-process cluster (auto self-signed SAN cert, enabled Caddyfile with `auto_https disable_redirects`, backend narrowed to 127.0.0.1:8080, devices enter via :8443); missing caddy/openssl ⇒ loud degrade to server-only on 0.0.0.0:8080. `start/restart --no-web` = single-process dev shape (embedded http). Live-verified: 2×running + `https://<lan-ip>:8443/healthz` round-trip + restart --no-web 1-app.
