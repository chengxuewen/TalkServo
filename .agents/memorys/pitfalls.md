# TalkServo Pitfalls

> Format (five-section):
> ```
> ## PIT-{n}: Title (date)
> - **Symptom**: [observed behavior]
> - **Root cause**: [root-cause analysis]
> - **Solution**: [correct approach]
> - **Verification**: [check command]
> ```
> Numbering starts at PIT-1 and increases consecutively. Every pitfall must map to an executable rule (see lesson-memory.md).
>
> Note: the documentation system was ported from MediaServo on 2026-09-28; its PIT-1~207 history is not migrated.
> MediaServo-domain pitfalls (libwebrtc/mediasoup/iceoryx2 etc.) are re-filed here only when they actually recur
> in this project, marked "sibling of MediaServo PIT-{n}".

## PIT-1: Subagent self-reports of "done" are untrustworthy; verify with grep against the filesystem (2026-09-28)
- **Symptom**: During the English-migration batch, 4 of 6 writing agents emitted complete "done" reports while repo-wide grep showed 0 changes landed on disk (CJK counts identical to the originals); another agent left edit-safety.md half-rewritten with garbage trailing content (229→123 lines).
- **Root cause**: Full-file rewrites of long files exhausted the agents' turns inside their thinking loop before any write tool call; resumed sessions contaminated by the failed pattern repeated it.
- **Solution**: After collecting any batch-rewrite results, first reconcile per-file violation counts with repo-level grep before deciding to resume or re-dispatch; re-dispatch with fresh sessions, one file per task, forcing the first tool call to be read/write; write critical files (those injected into instructions every turn) yourself.
- **Verification**: same command as the C1 check (conventions.md), which covers this ledger too

## PIT-2: Rewriting/squashing local history — use commit-tree, not rebase --root, when untracked runtime files collide (2026-09-28)
- **Symptom**: `git rebase --root -x "git commit --amend --reset-author"` aborted: "untracked files would be overwritten by merge" (`.omo/run-continuation/*.json` exist on disk, ignored but present; replaying commits that touch them conflicts). A follow-up `update-ref` appeared to fail because a stale terminal view still showed the old hash while HEAD had already moved.
- **Root cause**: rebase replays trees through the working directory; any on-disk untracked file inside a path the commits manage blocks checkout. Ignore rules do not protect against rebase checkout. Verification must read `git rev-parse HEAD`, not remembered output.
- **Solution**: For pure history rewrite (squash / author change) with a clean index, build the commit directly: `git write-tree` (or reuse an existing `<commit>^{tree}`) → `GIT_AUTHOR_*/GIT_COMMITTER_*` env → `git commit-tree` → `git update-ref refs/heads/main $new $old`. Never touches the worktree. Before `push --force`, confirm lost remote commits carry no unique content (`git log HEAD..origin/main` + `git diff --stat`).
- **Verification**: `git rev-list --count HEAD` (expected commit count); `git status --short` empty; `git ls-remote origin main` equals `git rev-parse HEAD`.

## PIT-3: Ported artifacts carry foreign-key references into a ledger that was reset (2026-09-28)
- **Symptom**: doc-audit found in ported skills/rules: 7 dangling C{n} refs (C4/C5/C6/C9/C13/C21), 5 refs to nonexistent PIT-{n}, one ACTIVE semantic clash (`[PIT-07]` = MediaServo's test-deletion lesson but reads against our PIT-1/2), ~40 unlabeled MediaServo PIT citations in edit-safety.md — and C2 itself was authored in Chinese, violating C1 from inside C1's own ledger.
- **Root cause**: the port/rename pass treated prose as language content, not as DATA containing foreign keys into a deliberately reset ledger. C/D/PIT citations are joins; reset invalidates all of them silently.
- **Solution**: on any cross-project port, script a referential-integrity pass: extract project-scoped identifiers (C/D/PIT ids, crate names, ports, doc paths), join against the target ledger, then re-anchor, delete, or label "MediaServo {id}". Never trust renamed prose.
- **Verification**: `grep -rn 'PIT-[0-9][0-9]\|C[1-9][0-9]?\b' .agents/skills/*/SKILL.md | grep -v MediaServo` cross-checked against `.agents/memorys` ids (audit join, rerun after any port).

## PIT-4: Standards/spec citations are agent output too — join-verify before trusting (2026-09-28)
- **Symptom**: audio-stack researcher reported "RFC 6716/8215/7254/8310/8311 full text fetched" and the frozen dossier cited RFC 8215 as the Opus FEC spec. media-reviewer fetched RFC 8215 = "Local-Use IPv4/IPv6 Translation Prefix" — the Opus authorities are RFC 6716 §2.1.7/2.1.9 + RFC 7587. "Fetched" in a report did not mean the content said what was claimed.
- **Root cause**: citation strings treated as trustworthy metadata rather than claims; lead never joined spec-ids to canonical titles during dossier write.
- **Solution**: for any external standard/RFC/section id an artifact cites, verify title (datatracker/curl) before commit; corrections to frozen dossiers go in a new dated file (done: architecture-review-media.md M-9), never back-write.
- **Verification**: `grep -rhoP 'RFC \d{4}' docs/reference/research/ | sort -u` then spot-check titles via https://datatracker.ietf.org/doc/rfcNNNN/.
## PIT-5: JSON-string wire ids read via Display embed quotes — three separate denials from one pattern (2026-09-30)
- **Symptom**: consume requests permanently denied; producer registry never matched consume ids on the live path (stub tests masked it — only the deny branch had coverage).
- **Root cause**: `serde_json::Value` variants carrying string ids were read with `.to_string()` (Display → `"id"` WITH quotes) in three places (Consume gate, ProduceOk registry note, media.rs consume) while registries stored bare ids.
- **Solution**: `value.as_str().unwrap_or_default()` everywhere a wire JSON string feeds an id comparison; added an ALLOWED-path stub test next to the deny one.
- **Verification**: `grep -rn "\.to_string()" crates/talkservo-server/src/ | grep -i "producer_id\|pid"` returns no Value-typed hits; live e2e consume flows (audible row) pass.
## PIT-6: mediasoup transports need a direction dimension — single-slot registries silently overwrite (2026-09-30)
- **Symptom**: producer score stayed empty (zero RTP uplink) even though both browser transports showed "connected"; audio only flowed one way or not at all.
- **Root cause**: server kept one transport per (room, peer); the recv transport overwrote the send one, so produce() landed on the wrong transport. E2E masked it for weeks because produce/announce/consume all "succeeded" as wire operations.
- **Solution**: wire carries `TransportCreate{dir}` / `TransportConnect{dir}` (additive, default "send"); transports keyed (room, peer, dir); producers bind to send, consumers to recv; peer_left cleans both.
- **Verification**: live e2e `holder audible` row passes with byte-growth assertion; producer score reaches 10 after resume.
## PIT-7: 0.0.0.0-bind + announced-address produced dead UDP candidates on this host (2026-09-30)
- **Symptom**: browser ICE stuck "connecting" forever; manual STUN to the worker port timed out; no error anywhere.
- **Root cause**: ListenInfo ip=0.0.0.0 with announced_address=Some(127.0.0.1) did not produce working sockets here (environment-specific mediasoup behavior).
- **Solution**: dev/e2e default binds 127.0.0.1 directly (dialable candidate, no translation); TS_ANNOUNCED_IP env switches to 0.0.0.0+announced for real deploys.
- **Verification**: `TS_E2E_FEATURES=live npx playwright test --grep "holder audible"` passes; transport connectionState reaches "connected" in the e2e log.
## PIT-8: netem on loopback impairs BOTH legs of the SFU media path — one-leg A/B measures cannot run on lo (2026-09-30)
- **Symptom**: FEC A/B showed 22.7% relative reduction (budget ≥30%) while the FEC switch, watchdog bypass and both arms' continuous streams all worked — the harness was correct, the number low.
- **Root cause**: `tc netem ... dev lo` applies loss to all loopback traffic; the SFU path traverses two legs on lo (holder→server, server→listener), each independently 12% — effective burst loss ≈22.6%. Opus inband FEC (1-frame redundancy) cannot recover consecutive losses, so the arm comparison measured the environment, not the codec.
- **Solution**: single-leg impairment A/B needs netns+veth (server in its own namespace, tc on one leg) or a real deployment (VPS WAN, one-leg tc). On lo-only, record numbers with the two-leg cascade caveat explicitly and defer budget decisions (done: fec-ab.json + architecture §4-8 annotation, decision moved to T5).
- **Verification**: `docs/acceptance/fec-ab.json` environment block declares the cascade; architecture §4-8 carries the measured annotation + deferral.
## PIT-9: env http_proxy captures Playwright/Chromium LAN connections — smoke/probe browsers need explicit direct routing (2026-10-08)
- **Symptom**: `net::ERR_CONNECTION_CLOSED` on every https://<lan-host> goto in playwright chromium, while curl -k to the same endpoint succeeds; ignoreHTTPSErrors and cert variants changed nothing.
- **Root cause**: chromium inherits `http_proxy/https_proxy/all_proxy` from the environment; LAN hostnames get proxied, the proxy refuses them (curl was tested only with `--noproxy "*"`). netlog showed `Proxy-Connection: keep-alive` — the tell.
- **Solution**: `--proxy-server=direct://` launch arg (or `proxy: { server: "direct://" }` context option); same discipline as the curl `--noproxy` rule, extended to browser automation.
- **Verification**: `web/scripts/lan-smoke.mjs` passes (TLS + WS + media setup over LAN); netlog contains no Proxy-Connection header when fixed.
## PIT-10: Plan-level wording can silently contradict a D-decision — measure the UX, align with the ledger (2026-10-08)
- **Symptom**: keydown→audible measured 600-1500ms per press (budget 300) — every press rebuilt the listener's consumer chain.
- **Root cause**: plan-3 wrote "AudioHandle released on FloorIdle/Taken/MediaDown (S-1)" and the SDK implemented exactly that; D13's text pins "consumers created-once-then-pause/resume-diff". Grant-exit release conflated resource cleanup with the confidentiality boundary — the boundary is the server-side producer PAUSE (D13), which already means zero downlink bytes while ungranted. Nobody caught it until a latency budget run quantified it.
- **Solution**: release triggers = producer owner left the room (roster) / MediaDown / teardown; grant-exit KEEPS the handle (track mute/unmute rides pause/resume). Unit test flipped to the D13 semantics; measured reused-path latency ~320ms.
- **Verification**: `packages/client` vitest "grant-exit KEEPS the handle" + live e2e latency harness (p50 ≤ 350 LAN budget); `grep -n "created-once" docs/modules/01* packages/client/src/media.ts` shows the pin comment.

## PIT-11: Idempotent init preserves stale topology — pre-D20 instances stay server-only after upgrade (2026-10-09)
- **Symptom**: `out/live/bin/talkservo-server start` prints `Apply complete: ... 1 unchanged` and `oxmgr list` shows only `talkservo-server` — no `talkservo-web` caddy leg — even though the D20 cluster-parity binary is in place.
- **Root cause**: `init_instance` deliberately preserves existing `run/oxfile.toml` and `identity/env` (operator property + no secret rotation). An instance initialized BEFORE D20 (degraded single-app render, `TS_BIND=0.0.0.0:8080`) is never re-rendered by later deploys, so `oxmgr apply` faithfully runs the stale spec.
- **Solution**: teardown + drop only the topology files, then redeploy: `talkservo-server stop <dir> && rm <dir>/run/oxfile.toml <dir>/identity/env && talkservo.sh deploy --lane live` — caddy+certs now available => `caddy in cluster` + 2-app oxfile + loopback backend bind. `etc/Caddyfile`, certs and `server.yaml` are untouched.
- **Verification**: `grep -c '\[\[apps\]\]' <dir>/run/oxfile.toml` must be 2; `oxmgr list` shows talkservo-server + talkservo-web; `curl -k https://<lan-ip>:8443/healthz` returns `{"ok":true}`.

## PIT-12: oxmgr app names are global — two instance dirs on one host overwrite each other (2026-10-09)
- **Symptom**: running `talkservo-server start out/stub` while `out/live` was up reported `Apply complete: 1 created, 1 updated` (not 2 created); later `stop out/stub` also tore down the LIVE cluster's apps.
- **Root cause**: `render_oxfile` emits fixed names (`talkservo-server`/`talkservo-web`) and a fixed namespace; oxmgr keys apps by name daemon-globally. A second instance apply redefines the same-name app to the other dir's command — and stop/delete sweeps both.
- **Solution**: single-instance-per-host is the current contract (sister D20 parity). Smoke/verify only ONE instance dir at a time; to re-check stub, tear live down first or keep stub to `run` mode (no oxmgr). Multi-instance support = app name/namespace derived from instance dir (needs oxmgr namespace scoping — not verified whether `apply` honors it); file as a decision before any fleet use.
- **Recurrence 2 (same session)**: a stub-lane smoke's `stop out/stub` (reading the OTHER instance's oxfile app names) stop+delete'd the live cluster's running apps — external hosts saw connection-refused until `start out/live` re-applied. Any per-instance verb against a second instance dir on a live host is destructive; single-instance contract means: never run stub+live cluster commands concurrently on one machine, period.
- **Verification**: `oxmgr list` shows exactly the app set of the instance you applied; a second `apply` of a different dir printing `updated` (not `created`) = collision, stop immediately.

## PIT-13: rustc linker "signal 7 [Bus error]" = / disk 100% full, not a code fault (2026-10-09)
- **Symptom**: `cargo test` fails at link with `linking with x86_64-conda-linux-gnu-cc failed ... ld terminated with signal 7 [Bus error], core dumped`; source/clippy otherwise clean.
- **Root cause**: `/dev/sda3` at 100% (59MB free) — ld cannot allocate output and dies on SIGBUS; the message never mentions disk, inviting a code/toolchain misdiagnosis.
- **Solution**: check `df -h /` FIRST on any linker/collect2 anomaly; reclaim space (here `rm -rf target/debug/incremental` freed 6.5G without touching `build/` C++ artifacts).
- **Verification**: `df -h / | tail -1` shows free >0, rerun compiles to `Finished`.

## PIT-14: stale-binary verification — `cargo test --lib` does NOT rebuild the bin target (2026-10-09)
- **Symptom**: deploy-layout probe of `talkservo-server stop` behaved with OLD semantics (cwd `.server`) while the new unit tests were green; md5 showed the probed binary was the pre-edit build (09:40).
- **Root cause**: after editing, only `cargo test --lib` ran (links the lib test harness only, not `src/main.rs` bin); the stale `target/debug/talkservo-server` was then copied for the live smoke.
- **Solution**: ALWAYS `cargo build -p talkservo-server` explicitly before any live/binary-level verification; copy only after timestamp/md5 confirms freshness.
- **Verification**: `md5sum <probe-path> target/debug/talkservo-server` identical AND probe output reflects the new semantics.

## PIT-15: answering an adjacent mechanism instead of the asked subject (2026-10-09)
- **One-liner**: user correction 「我现在说的是 start 命令，你为什么又提到 init」 — before answering, re-read the subject noun of the question; explain the neighbor mechanism only when the subject itself routes through it. <!-- c1:allow-zh -->

## PIT-16: gitignored instance dirs changed by unidentified means mid-session — silent secret rotation (2026-10-09)
- **Symptom**: (1) `out/stub/` vanished entirely between two command blocks (bin/ gone, dir absent — regenerable artifact, no source loss). (2) `out/live/identity/env` was regenerated at 13:33 with a NEW secret pair + `etc/server.yaml` replaced by a fresh-init render (the `admin_password_hash:` line added earlier was gone) — previously issued tokens invalid, the admin-password file lost. Nothing in the session log ran init/rm against those paths (`talkservo.sh build` only touches bin/ + web/; deploy preserves existing identity/etc).
- **Root cause**: UNATTRIBUTED — same machine hosts oxmgr daemons, e2e harnesses and multiple lanes; candidates (external cleaner, another agent session, oxmgr state ops) not proven. Treated as environmental.
- **Solution**: recovery pattern used: re-register accounts via the API with a surviving bootstrap token (`issue-admin-token.sh` + server env); store the initial password inside the SAME session immediately AND record `md5sum identity/env` in the working notes so a later silent rotation is detectable.
- **Verification**: `ls -la <dir>/identity/` mtime + `md5sum <dir>/identity/env` compared against the note — mismatch before any login debugging; `grep -c '^user:' etc/accounts.yaml` for registry presence.
- **Remediation landed (same day, revised by user)**: init now bootstraps `etc/accounts.yaml` (admin + plaintext password row, 0600, hand-editable, hot) so a wiped/fresh instance is login-capable in one step (sister T16 parity); `identity/env.fingerprint` (sha1) written alongside rotation. Instance location stays `out/live` per user ruling — out-of-repo prefix remains available if wipes recur. — `deploy --lane live --prefix ~/talkservo-live` (out/<lane> is staging again, disposable by design); `init` gained a loud WARN when no admin credentials exist (silent-503 class closed); `identity/env.fingerprint` (md5) written alongside every password rotation.

## PIT-17: "reference project X" requirements must be reproduced X-FORM, not my filtered weaker variant (2026-10-09)
- **Symptom**: admin auth went back twice in one day: user said 「参考 mediaservo」 -> I shipped password-ONLY login (no username) -> 「又错了，我想要的是 user/pwd」 -> shipped hash-only registry (no plaintext password) -> 「又错了，要求改为 mediaservo 那样的 accounts.yaml 中的账户和密码」. Three round-trips, two explicit rejections. <!-- c1:allow-zh -->
- **Root cause**: I substituted my own principles ("default secrets are an anti-pattern", "plaintext credentials never live in a config file") for the reference's documented behavior — and under-mined the evidence on the first pass: ms_rtc T16 (`frontend-process-split/tasks.md:162`: init GENERATES `accounts.yaml` + secret bootstrap, PIT-160 idempotent) and `unified-device-admin/proposal.md:14` (ops hand-EDITS accounts.yaml) existed from the start but I read only far enough to justify the minimal version. Each "safer-but-different" cut read to the user as another wrong implementation.
- **Solution**: for any 「参考 X / like X」 requirement, the proposal must cite X's **form triad** with source lines: (1) generation side (who creates the files — deploy/init?), (2) data shape (exact fields/format/permissions — plaintext vs hash?), (3) update mechanism (hand-edit+restart? API hot?). Reproduce that triad first; security improvements are ADDITIVE layers on top (our final shape: plaintext row supported AND hash row supported), never replacements of the asked form. <!-- c1:allow-zh -->
- **Verification**: before executing a reference-driven change, grep the plan for three citations (generator/shape/update-path + file:line). Missing any = not ready to build; mine the reference docs deeper.

## PIT-18: concurrent cargo invocations on the same target dir produce phantom compile errors (2026-10-09, second occurrence)
- **Symptom**: `pixi run lint` fails with "warning: build failed, waiting for other jobs to finish... could not compile (lib test)" while the same code passes tests seconds later; hit twice this session (13:5x after D22 live loop, 14:5x after stub suite).
- **Root cause**: e2e global-setup / test tasks run their own `cargo build` (different feature set) in the shared `target/`; a lint started while any cargo is still running races on feature-flavored artifacts — the errors name files that are actually fine.
- **Solution**: serialize: wait for in-flight cargo to exit, rerun lint — a rerun passing = phantom, do not "fix" clean code.
- **Verification**: before claiming a compile error is real, `pgrep -x cargo` must be empty; with a competitor running, rerun first, triagnose after.

## PIT-19: one-liner — grep/read output may TRUNCATE long lines with an ellipsis that is not in the file: `"ts-a…t"` was the real KEY bytes only once `od -c` said so; when a string literal matters (session keys, file paths), verify with `sed -n 'Np' file | od -c` instead of trusting displayed text.

## PIT-20: one-liner — shell `VAR=$(awk ... )` extracting a field that does not exist yields empty, and the curl then posts a malformed credential (401 read as "login broken"); when composing auth payloads from files, `printf '%s' "$VAR" | grep -c .` (non-empty) BEFORE trusting the status code — my 401 on the hash-row file was an empty `password:`, not a wrong password.

## PIT-21: Grant-set shrink emitted no rebroadcast — every mirror froze on the departed holder (2026-10-10)
- **Symptom**: Open-mode room with 3 concurrent web holders; one client released, but all pages (including its own) kept showing 3 speakers forever. Server FloorState was correct (2 holders) — only the wire broadcast was missing.
- **Root cause**: `floor.rs::promote_or_idle` had exactly two branches — "promote queue head → broadcast FloorGranted" and "grants+queue empty → broadcast FloorIdle" — an Exclusive-era dichotomy. D12 grant-set introduced the third shape (set shrinks, queue empty, holders remain): no branch emitted it, so peers never learned the set changed. The P0 multi-web e2e was the first test to exercise it.
- **Solution**: `else if !next.grants.is_empty()` branch broadcasting the current grant set (see floor.rs promote_or_idle); pinned by core test `open_concurrent_requests_all_granted_release_drops_only_that_peer`.
- **Verification**: `~/.pixi/bin/pixi run cargo test -p talkservo-core` (39) + `cd web && npx playwright test e2e/talk.spec.ts` (Speakers (3) → yield → 2 convergence asserts).
- **Prevention rule**: cross-client state assertions must observe **other peers' mirrors** (grants count/roster), not only the acting client's own button text — floor.spec's release assertion ("HOLD TO TALK") is client-side state and could not see the frozen grants (this is how the bug survived plan-3). Any grant-set-mutating path (release/leave/media-down/timeout) needs a hard "remaining holders re-broadcast" assertion.

## PIT-22: Multi-option adjudications via the interactive question tool got dismissed twice (2026-10-10/19)
- **Symptom**: two multi-option scope/UI decision points sent through the `question` tool (options with long Chinese descriptions) were dismissed by the user without an answer; re-presenting the same content as plain-text one-at-a-time adjudications got immediate rulings.
- **Root cause**: the question tool's compact interactive format cannot carry the seven-part detail (details/options with paired pros-cons/sources/impact/recommendation) a real adjudication needs; the user's established protocol for multi-option decisions is the adjudication-walkthrough skill (plain text, one per message) — using a different surface for the same job reads as noise, not as a question.
- **Solution**: any decision point with >=2 substantive options (design forks, scope rulings, walkthrough items) goes through the adjudication-walkthrough protocol — plain text, one adjudication per message, "Reply: Adjudication N — A/B/C". The question tool stays for simple confirmations (single yes/no execution gates) only.
- **Verification**: process-discipline item (not grep-able); recurrence check = another dismissed multi-option question tool call.
- **Prevention rule**: before sending ANY question with >2 options, switch to the walkthrough protocol; if the user asks for a plain-language re-render mid-walkthrough (the 说人话 cue), re-render the SAME adjudication in plain language and wait -- never batch the remaining items to catch up. c1:allow-zh
