# Deployment & Operations Handbook

> **Status (2026-10-23): SKELETON — adjudicated as the first slice of the next wave (full doc-audit 2026-10-23, user ruling #11).** Each section below names what exists today (with the command or file that proves it) and what is still owed. This is the GAP-5 home: the deferred items GAP-3 (secret rotation), GAP-4 (`audit.jsonl` ops story) and the rollout/limiting decisions flagged in modules/06 all live HERE until promoted into real sections. Living document — updated in place as slices land; nothing here claims a hosted runner (D26).

## 1. Deployment shapes (what exists)

| Shape | Entry | Artifact | Status |
|---|---|---|---|
| LAN dev loop | `pixi run cmake-probe && pixi run build && pixi run test` | `out/cargo-target/stub/debug/talkservo-server` (stub backend) | operating |
| Live single-host | `TS_LANE_LIVE=1 pixi run test-sfu` then `deploy/talkservo.sh` facade | versioned tar via `build:deploy` (bin + etc + scripts) | operating, LAN |
| Install staging | `cmake --install out/cmake-build --prefix <dir>` (components `bin`, `dist`) | `<dir>/bin/talkservo-server` + `<dir>/etc/*` + `<dir>/scripts/issue-token.sh` | operating |
| Compose front (D20) | `docker compose` + generated Caddyfile | hand-run parity/deploy image (D11/D26 — no hosted build) | spec'd; run by hand |

Instance model (D19-D21): the portable deploy unit is the instance directory — `etc/server.yaml` is the ONE operator file, `identity/` holds 0600 secrets only, `etc/Caddyfile` + `run/oxfile.toml` are generated. Facade: `talkservo.sh init/start/stop/restart/status/logs` supervising vendored `oxmgr`.

## 2. GAP-3 — Secret/identity rotation (OWNED HERE; no implementation yet)

Current truth:

- Admin auth (D22) mints JWTs from `etc/accounts.yaml` rows; accounts are hot-editable.
- The signaling/identity secret lives in `identity/` (0600); `crates/talkservo-server/src/auth.rs` notes "future rotation tooling".
- modules/06 advertises a rotation "ladder rung 3" at Alpha with NO vehicle — that promise is retired into this section: **rotation today = restart procedure**, and a restart invalidates outstanding join tokens.

Owed (adjudicated scope, write here before implementing):

1. **Restart-procedure rotation** (accept as the Alpha answer): steps to rotate the identity secret with a maintenance window; document that outstanding join tokens die with the old key and dispatchers must re-issue. Command: `talkservo.sh stop && <rotate identity/*.key> && talkservo.sh start`.
2. **Decision to record**: either (a) accept restart-rotation for Alpha (delete the "zero-downtime" expectation from modules/06), or (b) spec `kid`-keyed tokens (JWT header key id, dual-key verification window). Owner: next wave ruling.
3. **accounts.yaml hardening** (LOW, from audit F7): Alpha guidance = hash-only rows (the registry already accepts hash rows alongside plaintext; write the ops rule that production instances must not carry plaintext, and note the acknowledged timing asymmetry in `accounts.rs`).

## 3. GAP-4 — `audit.jsonl` operations (OWNED HERE; two real defects carried from the audit)

Current truth: `crates/talkservo-server/src/audit.rs` writes JSONL with day-based rotation (`audit-YYYYMMDD.jsonl`); the admin Logs tab pages by `seq` (plan-9 W2 push + 30s reconcile).

Owed:

1. **Seq cursor across restart/rotation (MEDIUM defect)**: `start_seq` is derived from the current file's line count, so after rotation+restart the sequence restarts and `before/after` cursors can collide across archive files. Spec fix (choose one, then implement): per-file ordinal + file identity in the cursor, or a monotonic base derived from a state file. Flag: duplicate/lost rows in Logs paging today.
2. **Retention (MEDIUM defect)**: rotated archives are never pruned. Ops rule to write: `audit_retention_days` knob or a documented external-logrotate contract (which glob, what restart does mid-rotation). Writer is fire-and-forget; failures surface only as `AUDIT_DROPPED` — say so here and in the runbook.

## 4. Entrance limiting — `/ws` per-IP budget (OWNED HERE; HIGH from audit)

Current truth: `/admin/api` and `/admin/ws` have per-IP connect budgets; the signaling `/ws` has cooldown/J-burst guards *after* connection but no pre-join per-IP limit. modules/06 self-flags this as O-F6 "one axum middleware at Alpha" — the deadline has arrived with the Alpha posture.

Owed: one axum per-IP middleware on `/ws` (connect + concurrent-rooms ceiling), mirroring the admin hub's bounded-budget pattern; document the knobs in the §config table. Blocks beyond-LAN exposure, not LAN use.

## 5. Media capacity — global UDP port budget (OWNED HERE; MEDIUM)

Current truth: `RTC_PORT_MIN/MAX` = 40000-40100 (~101 ports ≈ ~50 transports); per-room guardrail exists in `media.rs`, but there is no global counter, and exhaustion degrades to a raw worker error instead of `Error{media_capacity}` (E10).

Owed: one global in-flight transport counter + port-math paragraph here (rooms × transports vs pool), and E10 mapping for pool exhaustion. PoC single-room unaffected; Beta multi-room needs it.

## 6. Ops runbook stubs (write-on-encounter)

- Health: `GET /healthz` + compose healthcheck exist (`crates/talkservo-server/src/crash_recovery.rs` companion). Metrics export: NOT built (deferred, `obs.rs` is tracing/println today — acceptable PoC posture, revisited with the metrics slice).
- Deploy rollback: current answer is "redeploy previous versioned tar via `talkservo.sh`"; no automated rollback. Record any manual rollback here when first performed.
- Config migration: `server.yaml` precedence (process-env > identity sidecar > yaml > defaults, D21); schema changes so far have been add-only. First breaking change gets a migration subsection here.

## 7. Verification claims (D26 — nothing here runs remotely)

Every command above runs on the dev host by hand; the only automated enforcement is `.githooks/pre-push` → `scripts/gate.sh` and the pixi push obligations (`pixi run gate|test|e2e`). This file must never cite `ci.yml` as evidence (C5).
