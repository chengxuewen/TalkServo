# Gap Review — Rust lane (server, sfu, core)

> **FROZEN** — internal audit snapshot, 2026-09-30. Scope:
> `crates/talkservo-server/src/{room,ws,media,timers,turn,embed,config,auth,obs}.rs`,
> `crates/talkservo-sfu/src/{mediasoup_host,host,stub,lib}.rs`,
> `crates/talkservo-core/src/floor.rs` spot-checks, and the server/sfu test
> suites. Method: full read of every in-lane file; mediasoup API claims
> verified against the vendored `mediasoup-0.24.3` source in the local cargo
> registry. Findings ordered by severity within each focus area.

---

## Focus 1 — Concurrency / locking

**F1. Tokio Mutex held across `media.destroy_room().await` in the reaper — lock held over await, registry-wide stall window.**
- Severity: MED
- Location: `crates/talkservo-server/src/ws.rs:469-479`
- What: `IdleTick` calls `media.destroy_room(&state.id).await` **before** taking `app.rooms.lock()`, so that part is correct — but note the inverse hazard the task brief asked about: the lock is acquired only after the await and is released before the task returns (`drop(rooms)` then `drop(tx_probe)`). No lock is held across `destroy_room`. However, `main.rs:51-72` (W-sequence fanout) holds `rooms.lock().await` while doing an **unbounded number of `tx.send(...).await` calls** on a bounded (256) channel. If any room task is slow/busy (e.g. its `WatchdogTick` is awaiting `media_activity` against a wedged worker), every other room's join path stalls behind that lock.
- Why it matters: a single slow/blocked room task back-pressures the global room registry; new joins to *any* room queue behind it. Low probability in the stub backend, real in the live host where `media_activity` does a worker round-trip.
- Fix: collect the senders under the lock, drop the guard, then send outside the critical section (`let txs: Vec<_> = rooms.values().cloned().collect();` then loop). One-line-ish change; ordering is per-room anyway.

**F2. `spawn_room` timer loop leaks on channel-close; interval cadence is fine, lifetime is not.**
- Severity: LOW
- Location: `crates/talkservo-server/src/ws.rs:292-324`
- What: the four `tokio::time::interval` loops exit only when `tx.send` fails (all receivers dropped). That works — after the room task returns, its receiver drops, the channel closes, and the timer task exits on its next tick. But the *idle* interval is `cfg.room_idle_ttl_s.min(30)` — for the default TTL 600 s that is a 30 s tick firing `IdleTick` into the channel forever; fine. The subtle part: `floor_max_hold_ms.unwrap_or(u64::MAX)` — `tokio::time::interval` panics on zero-length periods; if an operator sets `FLOOR_MAX_HOLD_MS=0`, the server panics at spawn. Same class: `jwt_ttl_s` of 0 → `saturating_sub(300).max(1)` = 1 s, safe; `room_idle_ttl_s` of 0 → `min(30)` = 0 → **panic**.
- Why it matters: `env_u64` accepts any parseable u64 with no validation; a config typo takes the process down inside a spawned task (panic surfaces only as a dead timer loop, join-burst stops silently).
- Fix: validate in `Config::from_env` (fail fast: reject 0 for tick-derived durations), or `.max(1)` the idle interval like the refresh one.

**F3. `WatchdogTick` awaits `media.media_activity` per granted peer inside the single-writer loop — head-of-line blocking is accepted, but the peer loop re-locks per call.**
- Severity: LOW
- Location: `crates/talkservo-server/src/ws.rs:485-508`, `crates/talkservo-sfu/src/mediasoup_host.rs:377-397`
- What: acceptable single-writer design (media probes are awaited in-line; room protocol messages wait). Not a bug, recorded as the known latency lever. Note `media_activity` takes `producers.read()`, then `speaking.read()` — no lock nesting problem since both are `RwLock` reads and no other site holds one while acquiring the other in write mode (`apply_floor` takes `producers.read()` and awaits `producer.resume()` — an await under a read guard; that is fine for `RwLock` but it means a slow worker stalls other `media_activity` readers. Accepted at PoC; flag only.)

## Focus 2 — Lifecycle races

**F4. Reaper vs in-flight Join race: `or_insert_with` can return a dead sender — "room alive" `expect` can panic.**
- Severity: HIGH
- Location: `crates/talkservo-server/src/ws.rs:162-180`
- What: the join path clones the room sender from the registry, **releases the lock**, then does `room_tx.send(...).await.expect("room alive")`. Between the clone and the send, the room task can reap itself (empty room past TTL — the reaper removes the registry entry under the same-channel guard, then returns). If the registry entry was cloned just before removal, `send` fails on the closed channel and the `expect` panics the connection task (axum task panic → connection drops, server survives, but every racing join fails with a hard panic instead of a clean retry). The `same_channel` guard protects the registry, not the cloned sender in flight.
- Why it matters: this is exactly the reaper/Join interleaving the round claims to have closed; the registry can't be poisoned, but the error path is a panic rather than retry (`or_insert_with` a fresh room and resend, or treat send-failure as a retryable loop).
- Fix: on `send` error, re-check the registry: if the entry is gone or channel-diverged, re-run the `entry().or_insert_with(spawn_room)` step once and resend; drop the `expect`.

**F5. `reply` oneshot leaks a `cmd_rx` that is never used — dead channel + misleading comment.**
- Severity: LOW
- Location: `crates/talkservo-server/src/ws.rs:403-404`
- What: on successful Join the room task creates `let (_wire_tx, wire_rx) = mpsc::channel(64);` and returns `wire_rx` as the connection's per-connection command channel. But the room task never sends per-connection commands — all downstream traffic flows through the broadcast `Sink`. The pump loop at ws.rs:242-258 selects on `cmd_rx.recv()` which can only ever return `None` immediately-ish (sender dropped after... actually ` _wire_tx` is held alive until end of scope — it's kept alive until the Join handler scope ends, so `recv()` hangs forever, harmlessly). Dead code; the `Wire` arm in the pump loop (re-sending to self) is unreachable.
- Why it matters: misleading architecture surface (looks like a per-connection command path that does not exist); wasted channel.
- Fix: return a plain `()` ack in the oneshot, delete `cmd_rx` from the pump loop and the `RoomCommand::Wire` echo arm.

**F6. Watcher/handler leak in `router_for`: NOT a leak (verified against mediasoup 0.24.3 source).**
- Severity: NONE (informational — closes a review question)
- Location: `crates/talkservo-sfu/src/mediasoup_host.rs:112-125`
- What: `on_volumes`/`on_silence` return a `HandlerId` that is bound to `_volumes_handler`/`_silence_handler` and dropped when `router_for` returns. Checked vendored source (`audio_level_observer.rs`): `HandlerId` is an opaque ID; the callback `Arc` is owned by the observer's `handlers` registry (`self.inner.handlers.volumes.add(Arc::new(callback))`), so the closures live as long as the observer/router. The `_`-prefixed locals dropping does **not** unregister anything. Handlers survive correctly.
- Residual note: the volumes/silence closures hold a strong `Arc<SupervisorInner>` each; when `drop_router` removes the router and the observer is dropped, handlers are closed and the Arcs release — no cycle. The `speaking` map entries themselves, however, are only cleared by a *silence event or a later volumes event*: if a room is destroyed while a producer was actively above threshold, its `speaking` entry (keyed by producer id → room) stays in the map forever (producer ids are unique, so the entry is never reused). **Unbounded growth of dead entries across room churn.** See F7.

**F7. `speaking` map never cleaned on room destroy / producer drop — unbounded memory growth (E11 truth source).**
- Severity: MED
- Location: `crates/talkservo-sfu/src/mediasoup_host.rs:34-37, 112-125, 137-139, 410-412`
- What: `speaking: RwLock<HashMap<ProducerId, RoomId>>` is written by volumes events and only cleared by (a) silence events for the same room, or (b) `retain` on silence. `drop_router`/`destroy_room` and `peer_left` do **not** purge entries for the room. After reaping N rooms, the map holds N stale producer-id entries (each `RoomId` string). Producer ids are never reused, so nothing overwrites them.
- Why it matters: slow leak keyed by room churn; also a correctness smell — `media_activity` can never match a stale entry (its producer is gone) so behavior is unaffected, but any future `speaking.iter()` scan by room would see ghosts.
- Fix: in `destroy_room`/`drop_router`, `speaking.write().await.retain(|_, r| r != room)`. One line in `Supervisor::drop_router`.

**F8. `tx_probe` same_channel guard: sound, but the final `drop(tx_probe)` is a no-op — ordering comment is right, guard is correct.**
- Severity: NONE (informational)
- Location: `crates/talkservo-server/src/ws.rs:326, 472-482`
- What: the guard correctly refuses to remove a *successor* room's registry entry (spawned after this task died) — verified: `same_channel` compares sender identities, and a new room would have a different channel. The task also keeps the task-owned `tx` alive until return so the registry entry isn't the sole keeper — correct. `drop(tx_probe)` at line 480 is redundant (locals drop at return anyway) but harmless. One nit: the comment "its drop closes the channel for the registry" refers to `tx` — accurate.

**F9. Reaper emits `Leave` obs-event with peer `"-"` for room reaping — vocabulary abuse acknowledged in-code.**
- Severity: LOW
- Location: `crates/talkservo-server/src/ws.rs:462-468`
- What: the closed event vocab (modules/05) has no `room_reaped` event, so the code emits `leave` with a synthetic peer and a `reason` in `detail`. The comment admits it. Log parsers keying `event == "leave"` will count a phantom leave per reaped room. `Event::IdleTimeout` exists in the vocab but is unused everywhere — this was the intended event.
- Fix: emit `Event::IdleTimeout` (already in the closed set) instead of `Leave`.

## Focus 3 — Security

**F10. TURN timing-safe comparison question: N/A and sound — server is issuance-only, coturn does the verification.**
- Severity: NONE (informational — closes a review question)
- Location: `crates/talkservo-server/src/turn.rs:15-32`
- What: the server computes HMAC-SHA1 and hands credential + expiry-username to the client in `Welcome`; coturn's `use-auth-secret` recomputes the HMAC from the shared secret at allocation time. There is no server-side comparison to protect, so no timing side-channel exists on this process. Secret lives in `TURN_SECRET` env, physically separate from `TS_JWT_SECRET` (config.rs:33-42, no default ever for the JWT secret — `from_env` hard-exits). `issue()` returns `None` when secret/uri are empty and Welcome carries `{"uris": []}` — disabled-by-default is the right dev posture. Sound.

**F11. Frame-limit bypass: Join path checks `max_frame_bytes` after reading the frame, before parse — but the pump loop and the Join gate differ in error semantics, and there is no per-connection rate limit.**
- Severity: MED
- Location: `crates/talkservo-server/src/ws.rs:63-80, 207-212`
- What: both paths check `t.len() > max_frame_bytes` and close with 1009 — the limit itself is enforced. Two gaps: (a) axum's WS layer buffers the full frame into memory before the check, so the effective ceiling is whatever the server allows at the HTTP/WS layer; axum applies no default message cap in `WebSocketUpgrade` unless configured — worth a follow-up at Alpha hardening, not a PoC blocker. (b) Non-text frames (Binary/Ping/Pong) fall through the match with `_ => {}` in the pump loop (ws.rs:232) — fine — but the Join gate accepts *any* non-Text first frame as "return" silently, no close code, client sees an abrupt drop instead of an error message (minor UX/devx, not a bypass).
- Why it matters: the documented threat is DoS via oversized frames; memory blowup is bounded only by the WS layer, not by `MAX_FRAME_BYTES` alone. Recommend `max_message_size` on the upgrade in the same round it's cheap.
- Fix: `ws.on_upgrade` → configure `WebSocketUpgrade::max_message_size(app.config.max_frame_bytes)` (axum supports it), keeping the in-handler check as defense in depth.

**F12. JWT validation ordering: version-check → aud-blind decode → aud-strict re-validate. Correct, with one nit.**
- Severity: LOW
- Location: `crates/talkservo-server/src/ws.rs:115-130, 264-284`; `crates/talkservo-server/src/auth.rs:50-68`
- What: `extract_room_and_validate` decodes with `validate_aud=false` to read the room claim, then calls `auth::validate` with `set_audience(&[room])` + `validate_exp` + leeway 0. Signature and exp are checked **both** times (decode without aud still verifies sig+exp), so there is no trust window where claims are used unvalidated — the room string read in step 1 is only used to parameterize the strict validation in step 2, and the strict pass confirms `aud` matches. Correct two-step pattern. Nits: (a) `AuthError::WrongRoom` maps to generic `invalid_token` in the WS error surface (ws.rs:137-140) — acceptable; (b) `exp` is `u64` seconds and `validate` runs `now + ttl` — `issue` uses `now + ttl_s` without `saturating_add`; overflow needs ttl ≈ u64::MAX, unreachable via env parse? `env_u64` would happily accept `18446744073709551615`; `now + ttl` then wraps in release (panics in debug). Trivial: use `saturating_add` (auth.rs:40).

**F13. Hardcoded-secret scan: clean.**
- Severity: NONE (informational)
- Location: all in-lane files
- What: no `sk-`/`api_key`/`password=` patterns; the only literals are test secrets (`"test-secret-0123456789"`, `"turn-secret-0123456789"`) inside `#[cfg(test)]` and `format!("...-{}")` PID-scoped test secrets in integration tests — standard, not deployable credentials. `Config::from_env` never defaults the JWT secret (exits). `0.0.0.0:8080` bind default and `announced_address: None` in the live transport are deployment knobs, documented in modules/06, not secrets.

**F14. Media authorization: Consume has no grant/membership gate.**
- Severity: HIGH
- Location: `crates/talkservo-server/src/media.rs:77-88`; `crates/talkservo-sfu/src/mediasoup_host.rs:288-329`
- What: `TransportCreate` is guardrailed (count per room), `Produce` requires a prior `TransportConnect`, but `Consume{producer_id}` is forwarded to the backend with **no checks**: any joined peer in a room can (a) consume any producer id in that room — including a *paused* producer (transmission gating, D13, is bypassable at the signaling layer: the server-side consumer gets RTP parameters regardless of the grant set; the pause applies to the *uplink* producer, but a consumer opened server-side on a paused producer will still be created — mediasoup allows consuming paused producers, data just won't flow until resume... but the client gets valid `rtp_parameters` and can attach), and (b) consume **its own** producer. The floor model's rule is listeners hear only granted speech — with gating at the producer that's true at the RTP layer, but the D13 mechanism comment in host.rs says gating is the *diff on the producer*, so paused-producer consumption is harmless data-wise. The real issue: producer ids are obtainable (they appear in `ProduceOk` only to the producer... actually only the producer's own peer gets `ProduceOk`; other peers must guess ids — mediasoup ids are numeric strings, guessable in a small range in a fresh router).
- Why it matters: floor confidentiality (PTT is the product's core premise) currently relies on id-guessing being hard. Guessed producer id + Consume = a non-granted listener gets a valid consumer. modules/02 defines `Consume` as a client→server message with no stated gate; modules/01/D13 imply grant-scoped audibility — the server should at minimum verify the target producer exists in this room and (policy decision) refuse consuming paused producers to non-granted peers.
- Fix (minimal): in `handle_media_message` Consume arm, reject when the producer's peer is not currently granted (needs grant knowledge in MediaCtx or a query on FloorState) — or record producer_id→peer at ProduceOk and check `floor.is_granted(peer_of(producer))` before forwarding. Flag for user decision if the Alpha posture is "listen-only mode".

## Focus 4 — Spec conformance (modules/05 E-matrix, modules/01 rules)

**F15. `apply_floor` is never called by the server — D13 transmission gating is dead in the stub and live paths.**
- Severity: HIGH
- Location: `crates/talkservo-server/src/ws.rs` (Wire arm, lines 411-449), `crates/talkservo-server/src/room.rs` (handle_floor_message)
- What: modules/02 §P is normative: `grant broadcast + sfu.apply_floor(state)`. Grep confirms zero call sites in the server: neither the grant path (`handle_floor_message` → broadcast) nor MediaDown/Timeout paths invoke `media.apply_floor(...)` after a floor transition. Consequence: with the live backend, producers are created paused (mediasoup_host.rs:275) and **never resumed** — a granted holder stays inaudible. The stub suite can't catch it (stub `apply_floor` only records calls; no test asserts the server calls it). The P-press-to-audible acceptance flow (J-3-7 + grant → audible) is broken end-to-end on the live host.
- Why it matters: this is the single wire that connects arbitration to transmission (the platform's whole point). Tests pass because the stub records instead of enforces.
- Fix: in the room task after any floor-state change (`handle_floor_message`, `apply_media_down`, `apply_timeout`, ModeChange), call `media.apply_floor(&state.id, &state.floor).await` (and per modules/02 W-step 4, re-run it on `ProduceOk` — also missing).

**F16. `peer_left` never called by the server — media cascade (CM-4) dead in live path.**
- Severity: HIGH
- Location: `crates/talkservo-server/src/ws.rs:534-545` (Left arm), `crates/talkservo-server/src/room.rs:281-296`
- What: on `RoomCommand::Left` the server removes the member and purges the queue (core), but never calls `media.peer_left(&room, &peer)`. Live host: transports/producers/consumers for the departed peer are only reclaimed when the whole room is reaped. Stub: `StubState.rooms` keeps the peer. modules/03 §CM-4 and the `Sfu::peer_left` doc say the server calls it.
- Why it matters: every WS drop leaks a WebRtcTransport (a real worker-side object with ICE/DTLS state and a UDP port) until room TTL reaping; with `TRANSPORT_GUARDRAIL` 50 per room, a churny room exhausts the guardrail with ghost transports and blocks new joins.
- Fix: in the `Left` arm: `media.peer_left(&state.id, &peer).await;` before broadcasting the delta.

**F17. Spot-check rule 4 (preempt strict-greater): conformant. Rule 5 (ExceedsCeiling): NOT implemented. Rule 8 (idempotent leave/request): conformant.**
- Severity: MED (rule 5), NONE (rules 4, 8)
- Location: `crates/talkservo-core/src/floor.rs:173-197` (R4/R5-preempt), `floor.rs:154-168` (R4/R8 idempotent), absence of ceiling check in Request arm
- What:
  - R4 preempt: `*priority > holder_priority` strictly — matches modules/01 rule 4; equal/lower → `Denied{PreemptPriority}`, never silent. ✓
  - R8: Request by granted/queued peer echoes `FloorQueued` (granted echo is a silent no-op returning unchanged state — the normative text says "Repeated Request by grantee = idempotent"; a silent no-op is idempotent; queue echo present). Leave of unknown peer no-op. ✓
  - R5 (rule 5): `request.priority > user ceiling → Denied{ExceedsCeiling}` — there is **no user-ceiling model** in `FloorState` or the server; `FloorEvent::Request` carries only the client-supplied `priority` and no ceiling check exists anywhere (`DenyReason::ExceedsCeiling` is defined in wire.rs:29 but never constructed). `DenyReason::NotMember` and `NoMedia` are likewise never constructed. The closed-enum denials promised by modules/01 §"Priority model" are half-wired: any client can claim priority 255 and always win preemption.
  - Note: rule 5's ceiling is "static ceiling by role" — roles come from the JWT, so the server is the natural enforcer, not the core. This is a designed-in gap if PoC scope excluded it, but it is not marked YAGNI anywhere in modules/01 — the rule is normative and unimplemented.
- Fix: smallest honest step — document the deferral in modules/01 (annotate rule 5 as Alpha-scope with the review id), or enforce `priority <= role_ceiling` in the room task before forwarding `FloorEvent::Request`.

**F18. E11 semantics vs modules/05: watchdog probes granted peers only — matches spec; but the stub default `ActivityState::Active` for never-scripted peers makes the default path vacuous.**
- Severity: LOW
- Location: `crates/talkservo-sfu/src/stub.rs:230-238`; `crates/talkservo-server/src/ws.rs:485-508`
- What: E11 fires `MediaDown` only for *granted* peers (correct per F-matrix). Stub `media_activity` defaults to `Active` when nothing scripted — meaning in stub tests, silence never fires unless a test explicitly scripts it; the E11 code path in the room task is only exercised by `timers_flow.rs::e11_silence_past_grace...` which does script it. Acceptable, but the default choice silently biases toward "no E11" in any future test that forgets to script. `WatchState::forget` exists but is never called (granted peers that recover keep no state — fine; peers that *leave while silent* keep a stale `silent_since` entry until they re-grant... actually `probe` only iterates current grants, so a stale entry for a departed peer is never cleaned → tiny unbounded growth in `WatchState.silent_since` unless the peer is re-granted; `leave()` does not call `watch.forget`).
- Fix: call `watch.forget(&peer)` in the `Left` arm (one line); consider stub default `Silent` debate — leave as-is, documented.

**F19. Cooldown gate ordering (E8/rate-limit): server hardcodes 500 ms, ignoring `floor_request_cooldown_ms`.**
- Severity: MED
- Location: `crates/talkservo-server/src/room.rs:158-175`; `crates/talkservo-server/src/config.rs:50,73`
- What: `handle_floor_message` uses the literal `500` for the cooldown window; `Config.floor_request_cooldown_ms` (env `FLOOR_REQUEST_COOLDOWN_MS`, test value 500) is never read anywhere else. The config knob is dead. `timers_flow.rs` even comments "server cooldown is 500ms fixed (Config::for_test)". Hardcoded value = the exact anti-pattern the repo rules call out (`no hardcoded values` CRITICAL; requires `TODO:` marker minimum).
- Why it matters: acceptance tuning (modules/06 §config) is impossible without a code change; test comment documents the divergence knowingly.
- Fix: thread `cfg.floor_request_cooldown_ms` into `RoomState` at construction and use it in the comparison.

**F20. Obs coverage vs modules/05 vocabulary: most events never emitted.**
- Severity: LOW
- Location: `crates/talkservo-server/src/obs.rs:6-25`; emitters: `ws.rs:357,462,536`, `room.rs:207`
- What: the closed vocab declares 16 events; the server emits only `join`, `leave`, plus the `latency` line. `floor_request`, `grant`, `deny`, `taken`, `release`, `mode_change`, `transport_create`, `transport_fail`, `produce_ok`, `consume_ok`, `worker_exited`, `media_restart_done` (the last two via tracing in main.rs, which uses `tracing::info!` — those land in the JSON log via the tracing subscriber, but `obs::event` lines and tracing lines are two different formats: `obs` prints raw JSON with `println!`, tracing prints its own JSON envelope — **mixed log dialects on stdout**; acceptance parsers reading one format will miss the other), `rate_limited`, `idle_timeout` unemitted. The E-matrix says warn-level for denials/E-class — `obs::warn_event` exists and is never called; denials (FloorDenied broadcast) produce no log line at all, so the "#1 security surface" (E8 forged/invalid) has no audit trail.
- Why it matters: acceptance #7/#10 metrics and the E8 security audit read these logs. `grant_latency_ms` works (the one metric wired). `media_recovery_ms` has no `worker_exited` obs-event anchor in the live host (only a tracing line with event field inside its own envelope — grep-able, but the doc names obs events).
- Fix: wire `obs::event`/`warn_event` into the denial path (room.rs:201-220 on `FloorDenied`), media errors (media_error), and pick ONE log dialect for protocol events (route obs lines through tracing or drop tracing for protocol lines).

## Focus 5 — Test honesty

**F21. `latency_taps.rs` asserts NOTHING about latency lines — the name and comment claim a log line is verified; it is smoke-only.**
- Severity: HIGH (test honesty — the round's stated deliverable)
- Location: `crates/talkservo-server/tests/latency_taps.rs:30-75`
- What: `capture_stdout` is a stub returning `String::new()`; the comment inside admits it ("asserts the LOG by re-running the flow under `cargo test -- --nocapture` in CI grep"). Checked `.github/workflows/ci.yml`: **no `--nocapture` step, no grep of `grant_latency_ms` exists in CI** (jobs are plain `cargo test --workspace` / `cargo test -p talkservo-server --features sfu-mediasoup`). The claim "CI asserts the grep" is false — nothing asserts the latency line is ever printed. `grant_path_emits_latency_line` passes even if `obs::latency` were deleted. The queued-promotion test likewise asserts only the `FloorGranted` message, not the measured-from-original-request property the doc comment promises.
- Why it matters: modules/05 acceptance #7/#10 rest on these lines; the test suite reports green while the actual deliverable (the log line) is unverified. This is precisely the "verification honesty" violation pattern (claim in name, absence in assert).
- Fix (cheap and real): switch `obs::latency` to write via a pluggable writer (or have the test drive `room.rs`'s tap directly through a unit test on `RoomState::handle_floor_message` capturing the metric — the tap logic is pure), or add a real stdout-pipe capture in-process (`libc::dup2` or a `println` indirection). Minimum: make CI actually grep (`cargo test -p talkservo-server -- --nocapture | grep grant_latency_ms`) so the comment stops being false.

**F22. `reaper.rs` `TestServer.rooms` probe is dead scaffolding — the test never observes the registry.**
- Severity: LOW
- Location: `crates/talkservo-server/tests/reaper.rs:13-49`
- What: `TestServer` builds a *separate* rooms map, passes a fresh one into `AppState`, and `void_app_rooms` discards the probe — `room_count()` (line 41) counts a map nothing ever writes. The actual assertion uses observable behavior (re-join gets a fresh Welcome), which is honest — but the dead struct fields and the `void_app_rooms` no-op function are misleading scaffolding that suggests registry observation that does not exist. `room_count` is never called.
- Fix: delete `rooms`, `room_count`, `void_app_rooms` from the test harness (5 lines deleted, zero behavior change).

**F23. `live_e11_activity_truth_source` — semantics honest, assertion meaningful; one env-dependent escape hatch weakens both live produce tests.**
- Severity: MED
- Location: `crates/talkservo-server/tests/live_mediasoup.rs:92-135` (also `live_consume_across_peers` at 75-84)
- What: the E11 test's core assertion is real and well-named: apply_floor grants the peer (producer resumed), no audio ever flowed, `media_activity` must be `Silent` — this would fail under the old paused-state inference. Good. BUT: both `produce`-dependent live tests `return` (pass!) when mediasoup rejects the hand-built rtp_parameters, printing "produce rejected (env-dependent)". A regression in `produce` (e.g. options.paused set wrong, transport lookup broken) would surface as an early return and a **passing** test — the E11 truth source would never execute its assertion. "Env-dependent" is doing a lot of load-bearing work: on the designated Linux CI runner the minimal opus shape is deterministic; if it works once it works every run.
- Why it matters: the 10/10 live pass count may include silently-skipped bodies; CI cannot tell "produced and verified" from "produce rejected, skipped".
- Fix: keep the early return but mark it loudly: `eprintln!` + a sentinel like `panic!("SKIP: produce rejected")` is wrong for CI... the honest cheap option: fail on unexpected error *kinds* (parse/caps errors should never be env-dependent; only ortc validation might be), or assert `!router_caps().codecs.is_empty()` before produce so a caps regression fails first. At minimum, print `cargo:warning=`-style skip notice and count skips in the lane report.

**F24. `timers_flow.rs::e11_silence_past_grace_releases_and_promotes` — first `TestServer` construction (lines 94-98) is dead code including a no-op block; test itself is sound.**
- Severity: LOW
- Location: `crates/talkservo-server/tests/timers_flow.rs:91-116`
- What: the test builds a fast server, immediately rebuilds a second one with max-hold off (the `{ ... }` block at 96-98 does nothing, `srv.media = srv.media.clone()` is a self-assign), then shadows `srv`. The first server (bound socket, spawned task) leaks until the test process ends — harmless in-test but the dead code obscures the actual setup. The assertions themselves (MediaFailed to holder, promotion to queued peer) are real and match the E11/F-sequence.
- Fix: delete the first construction block; start directly with the max-hold-off config.

**F25. `cooldown_second_rapid_request_denied_rate_limited` — passes for the right reason but depends on an undocumented ordering (cooldown before idempotency); comment acknowledges it. Honest, fragile.**
- Severity: LOW
- Location: `crates/talkservo-server/tests/timers_flow.rs:189-210`; `crates/talkservo-core/src/floor.rs:159-168`
- What: the second rapid request hits the server cooldown *before* core's already-granted idempotent echo — the test asserts RateLimited arrives. If someone reorders the gates, the test starts failing spuriously (granted peer's request would echo nothing and the assert times out at recv budget). The in-code comment states the dependency. Acceptable with the comment; would be stronger with a dedicated non-granted requester. Not dishonest.

---

## Verdict summary

| Severity | Count | Findings |
|----------|-------|----------|
| HIGH | 4 | F4 (join/reaper panic), F14 (Consume unauthenticated), F15 (apply_floor never called), F21 (latency test asserts nothing) |
| MED | 6 | F1, F7, F11, F17, F19, F23 |
| LOW | 8 | F2, F5, F9, F12, F16→(HIGH actually), F18, F20, F22, F24, F25 |

*(Correction on the table: F16 `peer_left` never called is HIGH — 5 HIGH total: F4, F14, F15, F16, F21.)*

Blocking rationale: F15+F16 mean the live mediasoup path cannot carry audible audio or reclaim peer media objects — the two central plan-3 claims; F4 is a panic path on the exact race the reaper round claims closed; F14 is a floor-confidentiality hole; F21 is a test-honesty violation in the round's own deliverable.

rust lane: NEEDS (5 blocking)
