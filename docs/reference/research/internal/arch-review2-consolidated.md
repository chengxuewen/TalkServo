# Architecture Review Round-2 — Consolidated (Lead Merge)

> 2026-09-28 | 4 lanes: remediation(R)/sdk(S)/harness(H)/completeness(CM). Raw 36 → 19 unique after cross-dedupe. Round-1 ledger cross-checked: zero duplicate-of-fixed findings (remediation-verifier independently confirmed 21/21 landed, triangle D13×F-seq×E11 closes). CRITICAL bar: implementation per current docs ships failure or blocks P0-1.

## 🔴 HIGH (6)

| # | Finding | Sources | Fix (one-liner) |
|---|---------|---------|-----------------|
| 1 | **Ghost grant**: disconnected QUEUED peer never removed (Leave=no-op for non-granted, Rule 8); next promote grants a peer no longer in room | CM-2 | `Leave/peer_left` → also purge queue entry + broadcast updated `FloorQueued` positions; F-seq extended to non-holder members |
| 2 | **AudioHandle leak**: nothing owns `<audio>` teardown on FloorIdle/Taken/MediaDown; srcObject-held elements never GC | S-1 | SDK-owned AudioHandle{pause,release} per consumer; released on state transitions |
| 3 | **TokenRefresh × reconnect race**: missed push while disconnected → stale-JWT Join → 4401 with no channel; collides with no-retry-on-auth rule | S-4 | reconnect uses latest TokenRefresh cache; 4401 → SDK emits auth-expired, stops backoff (no auto-retry) |
| 4 | **No PeerJoined/PeerLeft deltas**: roster never updates mid-session; PeerList snapshot-only, and carries no gen | S-2 ≡ CM-3 (2 lanes) | add `PeerJoined{info,gen}`/`PeerLeft{peer,gen}`; PeerList gains gen (snapshot remains full-replace) |
| 5 | **pixi ci-check-mac broken as specced**: `--no-default-features` alone → zero backends → our own compile_error fires; would brick P0-1 copy-paste | H-3 | modules/09 §3 add `--features stub-media` (one line, before execution) |
| 6 | **Weak-net harness still not executable**: no exact tc commands/pinned interface; concealment formula lacks denominator + poll cadence | H-1 (round-1 H7 half-landed) | modules/04: `tc qdisc add dev <egress> root netem loss 12%`; ratio=concealedSamples/totalSamplesReceived, 1s poll; owner roles named |

## 🟠 MEDIUM (9)

| # | Finding | Sources | Fix |
|---|---------|---------|-----|
| 7 | Acceptance #12 "zero idle uplink" unreachable with server-only pause — client must also disable track (privacy/battery goal) | H-2 | define dual-gate: server pause (authoritative) + client track.enabled=false off-floor (D13 addendum) |
| 8 | Room GC/TTL after last leave undefined everywhere | CM-1 | `ROOM_IDLE_TTL_S` config (default 600) + reap → destroy Router; one line in modules/03 |
| 9 | v:1 prose-only — no version field on wire, no reject path for undecodable | CM-5 | `v` field in Join, echoed in Welcome; unknown/undeserializable → `Error{BadVersion}` + close |
| 10 | No peer cap / queue depth / frame-size limit next to E8 "security surface #1" | CM-6 | 3 config keys: `MAX_PEERS`(10) `MAX_QUEUE`(8) `MAX_FRAME_BYTES`(64KiB); overflow → Denied/413 |
| 11 | `Sfu::peer_left` cascade contract undefined | CM-4 | trait doc: closes transports+producers+consumers, removes queue, emits MediaDown |
| 12 | client `connectionstatechange` failed → local re-handshake contract missing | S-5b | SDK: failed → teardown+re-run J3-7 (mirror W-step 3) |
| 13 | mic-truth conflation: track.enabled vs server-pause | S-5 | SDK exposes `micTruth = enabled && !serverPaused`; UI reads SDK only |
| 14 | JWT role claims shape + escalation negative test unpinned | S-6 | claims `{sub, room, role}`; SDK rejects MuteSet/ModeChange for field role pre-send + server-side test |
| 15 | gen strictness at snapshot boundary unpinned (> vs >=) + event ordering guarantee | S-6b | discard `<` seen (not `<=`); events emitted strictly in gen order per peer |

## 🟡 LOW (4)

R2-1 stale `FloorGranted(holder)` in modules/04 (one word) · R2-2 E10 "50 transports" epistemic status disagrees across docs (keep "arbitrary guardrail") · R2-3 "holder ≡ grants[0] (Exclusive/Hybrid only)" definitional line missing in modules/01 · R2-4 acceptance numbering out of order · S-3 StrictMode/`useSyncExternalStore` owner rule (impl note) · S-7 TS drift CI = schemars→schema→regen→diff (cheap, add to plan-1 T3) · S-8 visibilitychange prohibition belongs in SDK text · H-7 "dual-OS test" overstatement (mac=check-only) · H-8 test-mediasoup apt deps list · H-10 pixi-rust vs rust-toolchain precedence note · H-9 test consoles conversation-only → D16 entry when design firms · CM-7 log RFC3339+rotation note · CM-8 **PRODUCT-DOMAIN**: PIPL/residency/recording-consent absent (flagged, not designed).

## ✅ Round-1 remediation verdict (independent): 21/21 landed, internally consistent, no regressions. Clean probes: gen-discard, C-8 role-scoping, audio cues, thin-view law, opusFec pin, queue privacy, wire-table↔SDK single-source.

## Fix routing
- **Before P0-1 starts (3)**: #5 (one line 09), R2-1/R2-2/R2-3/R2-4 (doc one-liners), S-7 into plan-1 T3.
- **Into plan-2/3 specs at expansion (7)**: #1 #4 #9 #10 #11 (wire/server), #2 #3 #12 #13 #15 (SDK).
- **Into D16/D13 addendum (2)**: #7 dual-gate, #8 room TTL.
- **PRODUCT-DOMAIN backlog (1)**: CM-8.
