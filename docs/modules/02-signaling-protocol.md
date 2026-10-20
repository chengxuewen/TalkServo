# Signaling Protocol — Wire & Sequences
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Wire contract (single WS enum, `#[serde(tag="type")]`, `v:1`)

| Direction | Messages |
|-----------|----------|
| client→server | `Join{jwt}` · `FloorRequest{priority,preempt}` · `FloorRelease` · `ModeChange{mode}`* · `MuteSet{peer,on}`* · `TransportCreate{dir?}` · `TransportConnect{dtls,dir?}` · `Produce{rtpParameters}` · `Consume{producerId}` · `Resync` |
| server→client | `Welcome{peerId,turnCreds}` · `RouterCaps{mediaCodecs}` · `PeerList` · `FloorGranted{grants,gen}` · `FloorTaken{by,gen}` · `FloorDenied{reason,gen}` · `FloorQueued{pos,gen}` · `FloorIdle{gen,reason?}` · `TransportInfo{ice,dtls,addrs}` · `ProduceOk{producerId}` · `ConsumeOk{producerId,rtpParameters}` · `MediaRestart{room,reason}` · `MediaFailed{peer}` · `TokenRefresh{jwt}` · `ServerSnapshot{...}` · `Error{code,detail}` |

`ServerSnapshot` is role-scoped (D12): field role receives `{mode,grants,muted,gen,peers}` only; the pending queue (intent-to-speak metadata) is dispatcher-only. `TokenRefresh{jwt}` is pushed proactively before expiry (LiveKit pattern) — client swaps silently; visible error only on refresh failure.

Additive-only evolution within a version; unknown fields rejected in debug, ignored in release. OpenAPI/JSON-schema of the contract is a `docs/reference/` living doc once core compiles.

## 2. Sequences (normative)

- **J join**: `WS → Join → Welcome+RouterCaps → TransportCreate(dir) ×2 → TransportInfo → TransportConnect(dir) → Produce → ProduceOk` — producer registered once, **server pauses it unless granted** (D13 transmission gating). `dir ∈ {"send","recv"}` (absent = "send", additive wire evolution): each client requests BOTH transports; the server keys them `(room, peer, dir)` — producers attach to the send transport, consumers to the recv one (measured: one-slot-per-peer silently overwrote, uplink scored empty — PIT-6). Identity collision: second Join presenting an already-connected identity → `Error{AlreadyJoined}` (PoC policy; displace-mode = Alpha decision).
- **P press-to-audible**: `keydown → FloorRequest → apply → grant broadcast + sfu.apply_floor(state) [diff: resume holder producer + resume/create matching consumers — mechanism unified in modules/03] → listeners audible`. Budget: ≤300 ms LAN / ≤600 ms public-relay (initial guess; measured at acceptance #10, then revised here).
- **X pre-empt**: strictly-higher request + preempt → old holder `FloorTaken{gen}`, all `FloorGranted{grants,gen+1}`, relay+gating swapped; non-strict → `Denied{PreemptPriority}`.
- **R reconnect (R1)**: `reconnect → Join → ServerSnapshot → client rebuilds; stale-gen events discarded`. No per-message ACK: WS runs over TCP (in-order within a connection); the only loss window is the connection boundary, which the snapshot exactly covers. (MCPTT re-send timers exist because its signaling rides unreliable transports — OQ-6 resolution, recorded.)
- **W worker restart** (normative steps; FloorState/generation untouched throughout):
  1. supervisor observes `worker.exited` → spawns replacement worker → rebuilds one Router per active room (same `mediaCodecs`)
  2. server broadcasts `MediaRestart{room, reason}` (payload: nothing else — clients already own the re-handshake script)
  3. each client tears down local transports and re-runs J-steps 3-7 (`TransportCreate → … → ProduceOk`) with its existing peer identity (same WS session; no re-Join)
  4. on each `ProduceOk`, the server re-runs `apply_floor(currentState)` — producers re-created paused unless granted; consumers re-created/resumed to match the grant set; new `ConsumeOk`s delivered
  5. room is audible again when all producers re-registered; expected downtime 2-4 s (measured at acceptance #7)
  DTLS/ICE ordering note: transports are fresh objects (new ufrag/pwd); browsers must `close()` old ones first (mediasoup-client lifecycle handles it).

- **F holder-media-failure (D12/E11)**: transport close / consent expiry (`iceConsentTimeout` 30 s default) or silent-RTP watchdog → `MediaDown{peer}` FloorEvent → grace `floor_media_grace_ms` → release + promote + broadcast; `MediaFailed{peer}` visible to all. Closes the zombie-holder path W/R/E4 cannot.

Server→client table footnote: `*` = authorized-role messages (D12); `Error` codes include `AlreadyJoined`.

## 3. Join credential — the `Join{jwt}` claims shape (plan-7 S4 debt payoff)

Written down 2026-10-20: this shape lived only in `crates/talkservo-server/src/auth.rs` (verified: the doc had zero claim-field mentions before this section).

HS256 over `TS_JWT_SECRET` (modules/06 ladder rung 1). Decode enforces `validate_aud` + `validate_exp` with `leeway = 0` — exact, deliberately not the library default 60 s grace.

| Claim | Wire type | Meaning | Enforcement |
|-------|-----------|---------|-------------|
| `sub` | string | peer identity, becomes the roster id | decoded, not validated |
| `room` | string | room the token was minted for | presence is load-bearing: `/ws` decodes THIS struct, so a claim set without `room` cannot pass |
| `role` | `"dispatch"` / `"field"` | wire `Role`, snake_case (modules/01 §1) | feeds the authority check (modules/01 Rule 10) |
| `aud` | string | room id, mirrors `room` | `set_audience([room])`; mismatch → `WrongRoom` |
| `mod_rooms` | string[] | rooms this peer may govern (S4, R3 ruling: authority is per-room, not global) | `serde(default)` + skipped when empty — **additive**: pre-S4 tokens decode to an empty list and keep verifying (R4 legacy-compat gate). Authority = `Dispatch role OR membership here`, OR-ed (a claim naming the token's own room is legal but redundant); validated at the issuance boundary only — `valid_id` charset + ≤64 chars, never inside `authorize_command` |
| `exp` | unix seconds | `now + ttl_s` at issue | expiry → `Expired`; `TokenRefresh{jwt}` re-mints proactively before it (§1) |

Audience isolation is STRUCTURAL, not a filter: the admin-console claim set deliberately carries **no `room` field**, so an admin token fails the `/ws` two-pass decode even under the misconfiguration where both secrets are equal; the REST side requires `aud:"admin"` + the separate `TS_ADMIN_SECRET`, so a user token gets 401 there (both directions covered by `permission_matrix` + the admin_api 401 rows).
