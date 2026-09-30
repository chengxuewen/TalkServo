# Gap-Closure Review — Consolidated (Lead Merge)

> 2026-09-29 | 2 lanes (rust / sdk-web) over plan-3 + gap round (8bade81..30af490).
> Raw 32 findings → merged below. Source dossiers: gap-review-rust.md,
> gap-review-sdk-web.md (both FROZEN).

## Verdicts

| Lane | Verdict | Blocking |
|------|---------|:---:|
| rust | **NEEDS** | 5 |
| sdk-web | **NEEDS** | 1 |

## 🔴 Blocking (fix before plan-4)

| ID | What | Why it matters |
|----|------|----------------|
| R-F15 | `apply_floor` never called by server | D13 gating dead on live host — granted holders inaudible; stub suite structurally blind (records instead of enforces) |
| R-F16 | `peer_left` never called on Left | WS drop leaks transports until room reap; guardrail 50 exhausts on ghosts |
| R-F14 | Consume has no grant/membership gate | guessed producer id → non-granted listener gets audio |
| R-F4 | join `.expect("room alive")` panics on reaper race | in-flight Join hits a self-reaped room's dead sender |
| R-F21 | latency_taps asserts nothing; no CI grep | the "measured" numbers are unverified |
| S-F4 | StrictMode double-invoke on CONNECT path | second transport → already_joined → reconnect storm; PTT dead in dev, receive-side masks it |

## 🟠 Medium (fix during expansion)

- R: rooms-lock held across fanout sends (main.rs W-path)
- R: speaking set unbounded on room churn (drop_router lacks purge)
- R: max_frame_bytes not set on axum upgrade (in-handler check only)
- R: rule-5 ExceedsCeiling unimplemented; DenyReason dead variants (NotMember/NoMedia) — decide implement vs YAGNI mark
- R: cooldown hardcoded 500ms, ignores floor_request_cooldown_ms
- R: live tests early-return PASS on produce rejection (silent skip)
- S-F3: already_joined doesn't set closedByUs → reconnect treadmill
- S-F6: pendingConsumes not cleared in teardown → ghost handles post-restart
- S-F10: e2e room/identity reuse flake vector; no retries config

## 🟢 Low / INFO

- S: diag.spec.ts in suite (no asserts, hard sleep) — remove
- S: fixed port 8090 in global-setup — go ephemeral
- S: SDK coverage 78.9% vs 80% bar (INFO)

## ✅ Cleared (verified, no finding)

TURN issuance-only timing-safety · observer HandlerId retention (vendored
source checked) · tx_probe same_channel guard · JWT two-step aud order ·
wire drift NONE (12 variants hand-checked) · gen-discard `<` · auth-expired
terminal (double-guard) · stable-snapshot/notify discipline · e2e
wire-coupling (roster-order cannot flake Preempt) · fixture replay strongest
drift guard · C1 clean both lanes.

## Fix routing

- **Now (plan-4 gate)**: R-F15, R-F16, R-F14, R-F4, S-F4, R-F21 — one pass
- **During expansion**: the 9 mediums (dead DenyReason = explicit decision)
- **plan-4 lane**: e2e retries/unique rooms, diag removal, ephemeral port, coverage bar
