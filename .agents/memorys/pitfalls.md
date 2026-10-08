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
