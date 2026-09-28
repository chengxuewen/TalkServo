# Signaling Protocol — Wire & Sequences
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. Wire contract (single WS enum, `#[serde(tag="type")]`, `v:1`)

| Direction | Messages |
|-----------|----------|
| client→server | `Join{jwt}` · `FloorRequest{priority,preempt}` · `FloorRelease` · `TransportCreate` · `TransportConnect{dtls}` · `Produce{rtpParameters}` · `Consume{producerId}` · `Resync` |
| server→client | `Welcome{peerId,turnCreds}` · `RouterCaps{mediaCodecs}` · `PeerList` · `FloorGranted{holder,gen}` · `FloorTaken{by}` · `FloorDenied{reason}` · `FloorIdle{gen,reason?}` · `TransportInfo{ice,dtls,addrs}` · `ProduceOk{producerId}` · `ConsumeOk{producerId,rtpParameters}` · `MediaRestart` · `MediaFailed{peer}` · `ServerSnapshot{state,gen,peers}` · `Error{code,detail}` |

Additive-only evolution within a version; unknown fields rejected in debug, ignored in release. OpenAPI/JSON-schema of the contract is a `docs/reference/` living doc once core compiles.

## 2. Sequences (normative)

- **J join**: `WS → Join → Welcome+RouterCaps → TransportCreate → TransportInfo → TransportConnect → Produce → ProduceOk` — producer stays live forever after (M1).
- **P press-to-audible**: `keydown → FloorRequest → apply → grant broadcast + sfu.apply_floor(state) → ConsumeOk per listener → audible`. Budget: ≤300 ms LAN / ≤600 ms public-relay (initial guess; measured in acceptance, then revised here).
- **X pre-empt**: strictly-higher request + preempt → old holder `FloorTaken`, all `FloorGranted{new,gen+1}`, relay set swapped.
- **R reconnect (R1)**: `reconnect → Join → ServerSnapshot → client rebuilds; stale-gen events discarded`. No per-message ACK: WS runs over TCP (in-order within a connection); the only loss window is the connection boundary, which the snapshot exactly covers. (MCPTT re-send timers exist because its signaling rides unreliable transports — OQ-6 resolution, recorded.)
- **W worker restart** (normative steps; FloorState/generation untouched throughout):
  1. supervisor observes `worker.exited` → spawns replacement worker → rebuilds one Router per active room (same `mediaCodecs`)
  2. server broadcasts `MediaRestart{room, reason}` (payload: nothing else — clients already own the re-handshake script)
  3. each client tears down local transports and re-runs J-steps 3-7 (`TransportCreate → … → ProduceOk`) with its existing peer identity (same WS session; no re-Join)
  4. on each `ProduceOk`, the server re-runs `apply_floor(currentState)` — consumers for the (possibly) still-valid grant set are rebuilt, new `ConsumeOk`s delivered
  5. room is audible again when all producers re-registered; expected downtime 2-4 s (measured at acceptance #7)
  DTLS/ICE ordering note: transports are fresh objects (new ufrag/pwd); browsers must `close()` old ones first (mediasoup-client lifecycle handles it).
