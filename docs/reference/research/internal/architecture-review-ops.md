# Architecture Review Dossier: Operational & Security Design (TalkServo PoC)

> **Status: FROZEN dossier (C2).** Internal architecture-review artifact, ops-reviewer lane of `arch-review-team`. Point-in-time; supersession annotated on the live docs (C2), never back-written here.
> Snapshot date: 2026-09-28. Method: read of `docs/modules/{02,03,04,05,06,08,09}`, `docs/architecture.md` §3 OQ, `docs/reference/research/{media/github-sweep-webrtc,ui/admin-ui-patterns}.md`; live verification of crate health via `crates.io` + `api.github.com`; vendor docs fetched live (LiveKit, Janus, coturn man page, mediasoup.org, 12factor.net, MDN).

## 1. Findings summary

| # | Area | Current design (source) | External precedent | Verdict |
|---|------|------------------------|--------------------|---------|
| F1 | JWT signing key | Single HS256 secret `TS_JWT_SECRET`, TTL 3600 s, no rotation, no `kid` (06 §2) | LiveKit: `api_key`+`api_secret` **pair** — key id in token, multiple keys per project, revocable (tokens-grants docs) | GAP (Alpha): adopt key-id + dual-secret to enable rotation; PoC-acceptable |
| F2 | Dev token minting | `scripts/issue-token.sh` signs with the same secret that validates joins (06 §2) | LiveKit dev token server: "useful for development and testing but insecure in production" (auth docs) | OK — boundary IS documented ("admin-issued at Alpha"); add "never expose outside LAN" note to script |
| F3 | Admin vs signaling auth | Split rule adopted now, surface at Alpha (08; D196 lineage; admin-ui-patterns §2) | Janus `admin_secret` on separate path; LiveKit Cloud keys | CONSISTENT |
| F4 | WS transport security | 06 covers SRTP only — **no `wss://`/TLS termination story anywhere** | Browsers require secure context for `getUserMedia` — non-HTTPS page gets `navigator.mediaDevices === undefined` (MDN); talktome ships HTTPS self-signed; MediaServo had Caddyfile | GAP (PoC-blocking for acceptance #9 public deployment): even a self-signed/Caddy section in 06 is needed — public ICE-class test is physically impossible over plain `ws://` from a browser |
| F5 | TURN credential window | HMAC short-TTL creds, 3600 s default, delivered in `Welcome` (04 §1, 06 §2) | coturn REST API: username=`timestamp:username`, password=base64(HMAC(usercombo, secret)) via `--use-auth-secret`+`static-auth-secret`; long-term mech requires `--lt-cred-mech` + realm `-r`; `--stale-nonce` default nonce lifetime **600 s** (turnserver.1) | TRAP: 06/04 never name the coturn flags/realm the compose file must set; 1 h cred TTL vs 600 s nonce window is fine only because browsers re-challenge on 401 — but an expired timestamp mid-allocation-refresh yields auth failure. Pin `use-auth-secret`, `static-auth-secret` (env), `realm`, and TTL alignment in `docker/coturn.yml` at P0-1 |
| F6 | Signaling flood defense | Floor-request cooldown 500 ms only (06 §2); E8 log+reject, no bans (06 §2) | Janus: rate-limiting + `accept_new_sessions` drain; LiveKit Cloud quotas | GAP (cheap): no per-IP connect/join rate limit or concurrent-connection cap; document as accepted PoC risk or add one axum middleware at P0-1 |
| F7 | Crash consistency / drain | E7: server death = all rooms lost, "PoC: no persistence — accepted" (05 §1); no SIGTERM graceful drain anywhere | Janus drain mode; talktome relies on Docker `--restart unless-stopped`; 12-factor: disposability = fast start, **graceful shutdown** | PARTIAL: crash loss is a deliberate accepted decision (consistent), but planned restart (deploy/config) also drops rooms — SIGHUP/SIGTERM drain (stop accepting Join, broadcast, exit) is an Alpha item; put it in 06 §1 ladder next to split-services |
| F8 | Health endpoint | **No `/health` or liveness route on the server surface** (03: "axum+WS, room mailbox, timers, rate limit"; 06 silent) | LiveKit and Janus both expose health (`get_status`/`info`/`ping` admin API); MediaServo ships `/health`; coturn has telnet CLI + Prometheus | GAP (PoC, trivial): one axum `GET /healthz` — needed by docker compose healthcheck and CI e2e readiness gate; recommend adding to 03/06 |
| F9 | Observability | Structured JSON logs only, closed event vocabulary, acceptance metrics derived from logs (05 §Observability) | LiveKit Analytics API, Janus event handlers; metrics = Alpha scope | CONSISTENT; vocabulary is machine-parseable so a CI log-metric extractor is viable; note: coturn itself exports Prometheus (port 9641) for free TURN-side ops data |
| F10 | Backup/restore | N/A — no persistence by design (05 E7, "explicitly NOT handled") | — | CONSISTENT (nothing to back up) |

## 2. Supply-chain verification (live API, 2026-09-28)

- **`mediasoup` Rust crate = official upstream, not a third-party side project.** Repo layout: `rust/` directory inside `versatica/mediasoup` (alongside `node/`, `worker/`); `doc/Rust-crates.md` documents the 3-crate split (`mediasoup` / `mediasoup-sys` / `mediasoup-types`). Crate 0.28.1 published 2026-09-16; 273 k downloads; repo pushed same-day, 7 378 stars, 55 contributors.
- **Concentration risk**: top contributors `ibc` (3 247 commits), `jmillan` (622), `nazar-pc` (273) — effective bus factor ≈3, all tied to the Versatica company; same lineage MediaServo bet on. Node-sidecar alternative is now moot: the Rust port is maintained by the same team as the C++ worker.
- **Version drift trap**: D6/03 pin "crate 0.24"; upstream shipped 0.24.3 → 0.28.1 (5 minor bumps) in ~7 weeks. Pre-1.0 semver means every minor may break API. Mitigation exists (Cargo.lock committed, 03 §layout) — add an upgrade-cadence note to 09: lockstep with worker protocol version, batch upgrades behind `pixi run test-sfu`.
- **`webrtc-audio-processing` (D4)**: `tonarino/webrtc-audio-processing`, 330 stars, 11 contributors (top 5: strohel 68, jacksongoode 33, skywhale 29, bschwind 23, mcginty 22), crate 2.1.0 (2026-05), 134 k downloads, pushed 2026-07-16, **BSD-3-Clause**. Community-scale but healthy; PoC web path uses browser-native 3A (04 §1), so exposure is deferred to Beta native clients — risk window is real but not PoC-critical.
- **Licenses**: mediasoup (C++ + Rust) **ISC**, webrtc-audio-processing **BSD-3-Clause** — permissive, compatible with the workspace; `deny.toml` + `pixi run audit` land at P0-1 (09 §2) — gate exists on paper, wire it into CI job list.

## 3. Config & pinning posture

- 12-factor alignment: secrets via env (`TS_JWT_SECRET`, fail-fast startup validation in 06) matches the "store config in the environment" doctrine; no config-in-repo drift found. `.env` sample + `scan-hardcode.sh` rule lands P0-1 (09 §4) — adequate for PoC.
- Pinned: `rust-toolchain.toml` ✓ (03/09), `Cargo.lock` committed ✓ (constraints rule). **Gap: `pixi.lock` commit rule is never stated** (09 lists `pixi.toml` but not the lockfile) — add one line to 09 §5; without it, meson/clang/openssl versions float across machines, which is exactly the mediasoup-sys build pitfall surface (09 §2 MESON_ARGS row).

## 4. Recommended actions (severity-ordered)

1. **HIGH / PoC-blocking before public acceptance test (#9)**: write the transport-security line into 06 — TLS termination (Caddy or axum-rustls), `wss://` + `https://` page origin; browsers gate mic on secure context (F4).
2. **MEDIUM / P0-1**: `GET /healthz` on the server surface + compose/CI healthcheck (F8); pin coturn flags incl. `realm` + `use-auth-secret`/`static-auth-secret` in `docker/coturn.yml` and cite them in 04 (F5).
3. **MEDIUM / Alpha backlog explicit**: JWT key-id + rotation design (F1), SIGTERM drain + restart policy (F7), per-IP connect/join limit (F6).
4. **LOW**: pixi.lock commit rule (one line in 09); "dev-only, LAN-only" banner on `issue-token.sh` (F2); upgrade-cadence note for the mediasoup crate (F2 §2 drift).

## Sources

- LOCAL (read 2026-09-28): `docs/modules/02,03,04,05,06,08,09`, `docs/architecture.md`, `docs/reference/research/media/github-sweep-webrtc.md`, `docs/reference/research/ui/admin-ui-patterns.md`.
- https://crates.io/api/v1/crates/mediasoup (+/0.28.1, /webrtc-audio-processing, /mediasoup-sys) — versions, dates, downloads, repo fields, fetched 2026-09-28.
- https://api.github.com/repos/versatica/mediasoup (+/contents/, /contents/rust, /contributors) — ISC license, 55 contributors, `rust/` dir present; https://api.github.com/repos/tonarino/webrtc-audio-processing (+contributors) — 11 contributors, BSD-3-Clause; all 2026-09-28.
- https://raw.githubusercontent.com/versatica/mediasoup/master/doc/Rust-crates.md — official 3-crate architecture.
- https://raw.githubusercontent.com/coturn/coturn/master/man/man1/turnserver.1 — `--use-auth-secret`, `static-auth-secret`, `--lt-cred-mech`, realm, `--stale-nonce` 600 s default, Prometheus/telnet management surface (fetched 2026-09-28).
- https://docs.livekit.io/home/get-started/authentication/ + https://docs.livekit.io/frontends/reference/tokens-grants/ — api_key/secret pair model, dev token server "insecure in production" wording (fetched 2026-09-28).
- https://janus.conf.meetecho.com/docs/ — docs index (no dedicated security page in multistream doxygen; admin_secret/drain facts via admin.html, already verified in `ui/admin-ui-patterns.md` §1.3).
- https://12factor.net/config — config-in-environment doctrine.
- https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getUserMedia — secure-context requirement (mic blocked on non-HTTPS origins).
- mediasoup v3 API docs (`worker.on("died")`, `worker.died`) — crash-event surface backing 05 E6 detection design.
