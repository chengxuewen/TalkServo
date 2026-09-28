---
name: api-interface-design
description: "Contract-first design for TalkServo: Rust traits (Component/Plugin), WebSocket signaling protocol (SignalingMessage enum), and REST API boundaries (OpenAPI 3.0.3). Enforces protocol backward compatibility, crate-boundary contracts, and serde wire-format discipline. Use BEFORE adding new API endpoints, WS message types, or crate-level trait changes."
---

# api-interface-design — Contract-First API Design

> Define the contract BEFORE the implementation. Traits, messages, and endpoints are the architecture — code is decoration.

## TalkServo API Boundaries

TalkServo has three distinct API contract surfaces:

| Boundary | Form | Location | Contract |
|----------|------|----------|----------|
| **Component/Plugin traits** | Rust traits | `talkservo-media` (engine/), `talkservo-host` (host/) | Trait signature stability |
| **WebSocket signaling** | JSON enum | `talkservo-common/src/protocol.rs` | `#[serde(tag = "type")]` discipline |
| **REST API** | OpenAPI 3.0.3 | `docs/openapi.yaml` | Schema + validation |

## Design Protocol

### Phase 1: Scope the Change

Which boundary is affected?

```
Component trait change → talkservo-media traits
WS protocol change    → talkservo-common protocol.rs
REST endpoint change  → OpenAPI spec + server routes
Cross-boundary        → Draft all contracts FIRST
```

### Phase 2: Write the Contract

**Rust Trait Contract:**

```rust
// ✅ CORRECT: minimal, composable trait
pub trait MediaComponent: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(&mut self) -> Result<(), ComponentError>;
    fn stop(&mut self) -> Result<(), ComponentError>;
}

// ❌ WRONG: implementation leaks, too many methods
pub trait MediaComponent: Send + Sync + Debug + Clone { /* 15 methods */ }
```

Rules:
- One trait = one responsibility
- Use `thiserror` for error types (library crates)
- `&str` over `String` for params (borrowing)
- `&[T]` over `Vec<T>` for collections
- Never expose internal types in public API
- `#[non_exhaustive]` on enums meant for extension

**WebSocket Protocol Contract:**

```rust
// ✅ CORRECT: tagged enum, snake_case, backward-compatible
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    // Existing variants unchanged
    RoomJoin { room_id: String, peer_role: PeerRole },
    
    // NEW: additive only, no field renames
    StreamStats { room_id: String, fps: f64, bitrate_kbps: u64 },
}

// ❌ WRONG: camelCase tags (browser mismatch PIT-06), removed fields
#[serde(tag = "type")]  // missing rename_all → camelCase default
pub enum SignalingMessage {
    RoomJoin { room_id: String },  // peer_role removed = breaking
}
```

Rules:
- **ALWAYS** `#[serde(tag = "type", rename_all = "snake_case")]`
- Browser clients MUST send snake_case type tags (PIT-06)
- Additive changes only — never rename/remove fields
- New variants append to the END of the enum
- `Option<T>` for new fields on existing variants
- Protocol changes → update E2E test scripts

**REST API Contract (OpenAPI 3.0.3):**

```yaml
# docs/openapi.yaml
paths:
  /api/health:
    get:
      summary: Server health check
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/HealthStatus'

components:
  schemas:
    HealthStatus:
      type: object
      required: [status, uptime_seconds]
      properties:
        status:
          type: string
          enum: [ok, degraded, down]
        uptime_seconds:
          type: integer
```

Rules:
- OpenAPI spec is the source of truth for REST
- CI validates spec (`openapi-validate` job)
- Every response has a schema
- Every endpoint has error responses (4xx, 5xx)
- Server routes mirror spec paths exactly

### Phase 3: Backward Compatibility Check

| Change | WS Protocol | REST API | Rust Trait |
|--------|:-----------:|:--------:|:----------:|
| Add new enum variant | ✅ Safe | — | — |
| Add new field (optional) | ✅ Safe | ✅ Safe | — |
| Add new trait method (default impl) | — | — | ✅ Safe |
| Rename existing field | ❌ BREAKING | ❌ BREAKING | — |
| Remove enum variant | ❌ BREAKING | — | — |
| Change field type | ❌ BREAKING | ❌ BREAKING | ❌ BREAKING |
| Remove trait method | — | — | ❌ BREAKING |

## Cross-Boundary Rules

### Component → WS Protocol
- Component errors must not leak into WS messages
- Map internal errors to `SignalingMessage::Error { code, message }`
- Error codes: 1xxx = client, 2xxx = server, 3xxx = SFU

### WS Protocol → REST
- WS message types and REST response schemas are distinct
- Don't reuse WS enum variants as REST response types
- REST uses snake_case JSON keys by default (like WS)

### Crate Dependency Direction
```
talkservo-common  ← protocol types (leaf dependency)
        ↑
talkservo-media   ← component traits
        ↑
talkservo-server  ← implements traits, handles WS
talkservo-host    ← implements traits, sends WS
talkservo-client  ← implements traits, sends WS
```

No circular dependencies. The protocol crate has zero internal deps.

## Verification Gates

### Per Boundary

```
[ ] Rust trait:  cargo doc --no-deps -p talkservo-media  (check docs compile)
[ ] WS protocol: cargo test -p talkservo-common           (serde roundtrip tests)
[ ] REST API:    python3 -c "import yaml; yaml.safe_load(open('docs/openapi.yaml'))"
[ ] E2E:         Host → WS → Server → WS → Client test scripts pass
```

### Additive Change Checklist

```
[ ] No existing fields renamed or removed
[ ] New fields use Option<T> when adding to existing variants
[ ] New variants appended to end of enum
[ ] serde tag/rename_all unchanged
[ ] E2E test scripts updated for new messages
[ ] Backward compat: old client ignores unknown variants (serde default)
[ ] Error codes assigned from correct range
```

### Breaking Change Protocol (IF UNAVOIDABLE)

1. Draft the breaking change in `docs/modules/protocol/`
2. Version the WS endpoint: `/ws/v2` alongside `/ws`
3. Deprecation window: 1 release cycle
4. Migration guide in `docs/modules/protocol/migration-v1-to-v2.md`

## Common Pitfalls

| Pitfall | Symptom | Fix |
|---------|---------|-----|
| camelCase WS tags | `CreateWebRtcTransport` ignored | snake_case only (PIT-06) |
| Missing `peer_id` in SFU messages | Transport not found | All SFU msgs need peer_id (PIT-08) |
| Internal error in WS message | Crash, not Error response | Always map to `Error { code, message }` |
| Renaming protocol field | Old clients break silently | Never rename; add new field instead |
| Trait method without default | All implementors break | Add default impl when possible |

## Related Skills

| Skill | Relationship |
|-------|-------------|
| `context-engineering` | Which crate gets which protocol change |
| `test-harness` | Generate serde roundtrip + E2E test skeletons |
| `review-hardcode` | Scan for hardcoded ports/URLs in new endpoints |
| `think-before-act` | Contract review BEFORE implementation |

---

## Hyrum's Law

> With a sufficient number of users of an API, it does not matter what you promise in the contract: all observable behaviors of your system will be depended on by somebody.
>
> — Hyrum Wright, Google

**Every observable behavior becomes a de facto contract.** This matters especially in TalkServo:

| Boundary | Observable behavior (implicit contract) | Guard |
|------|----------------------|------|
| WS protocol | Message ordering, field timing, connection retry interval | E2E scripts pin the behavior → change it and they fail |
| Rust trait | Method call order, Send/Sync impls, error types | `#[non_exhaustive]` + default impls |
| REST API | Response time (~P50), JSON key ordering, empty array vs null | OpenAPI spec pins the contract |
| Cargo features | Which crates depend on which feature combinations | CI matrix tests all combinations |

**Principle**: if you change the API, users break — whether or not you consider it an "internal detail". Add, don't remove.

---

## One-Version Rule

> In a crate workspace, every dependency should resolve to exactly one semver-incompatible version. Never allow diamond dependencies.

### Project Constraints

TalkServo is a 7-crate workspace:

```
talkservo-common  ← leaf, no workspace deps
        ↑
talkservo-media / talkservo-codec / talkservo-webrtc
        ↑
talkservo-host / talkservo-client / talkservo-server
```

Rules:

| Rule | Reason |
|------|------|
| All workspace crates use the same `[workspace.dependencies]` versions | Avoid linking `serde 1.0` and `serde 2.0` at the same time |
| No concrete version numbers in `[dependencies]` | Versions are defined only in the workspace root `Cargo.toml` |
| Run `cargo tree -d` before adding any external dependency | Confirm no duplicate dependencies are introduced |
| Third-party crate upgrades must be consistent across the workspace | Upgrading `tokio` means the whole workspace upgrades together |

**Check command**: `cargo tree -d --workspace` — empty output = pass.

---

## Branded Type IDs

> Use wrapper types instead of raw `String`/`u64` for domain identifiers to prevent accidental misuse at the type level.

### Pattern

```rust
// ✅ CORRECT: branded types — no accidental RoomId → PeerId swap
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransportId(String);

fn connect(room: RoomId, peer: PeerId, transport: TransportId) { /* ... */ }
// RoomId, PeerId, TransportId are not interchangeable — the compiler blocks misuse
```

```rust
// ❌ WRONG: all String, easy to swap arguments
fn connect(room_id: String, peer_id: String, transport_id: String) { /* ... */ }
```

### Benefits

| Unbranded | Branded |
|----------|--------|
| `fn route(room_id: String, peer_id: String)` — swapped arguments fail silently | `fn route(room_id: RoomId, peer_id: PeerId)` — the compiler rejects |
| `HashMap<String, Transport>` — ambiguous keys | `HashMap<TransportId, Transport>` — explicit keys |
| ID semantics lost on serialization | Display/Serialize preserve type semantics |

### Project Convention

TalkServo key ID types (branding recommended):

```
talkservo-common/src/protocol.rs:  RoomId, PeerId, SessionId
talkservo-media/src/engine/:      StreamId, TrackId
talkservo-server/src/sfu/:        TransportId, ProducerId, ConsumerId
```

> **ponytail**: brand only IDs that cross module boundaries. Internal one-shot local variables don't need it.
