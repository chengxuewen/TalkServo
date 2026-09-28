---
name: code-simplification
description: "Reduce complexity in TalkServo Rust/TS code. Chesterton's Fence analysis, Rule of 500 enforcement, dead code elimination, borrow checker simplification patterns. Complements ponytail for Rust-specific over-engineering. Use after ponytail-audit, before PRs, or when diagnosing complexity smells."
---

# code-simplification: complexity reduction

> Chesterton's Fence + Rule of 500 + Rust borrow checker patterns.
> Only delete what should be deleted. Never delete what you don't understand.

## Trigger conditions

- User says "too complex", "simplify", "refactor", "dead code"
- ponytail-audit found removable items
- File exceeds 500 lines (`wc -l` > 500)
- `cargo clippy -- -W clippy::cognitive_complexity` reports high complexity
- Function exceeds 50 lines (`wc -L` > 50 per function)

## Chesterton's Fence workflow

```
Find complexity → investigate cause → valid reason? → YES: keep, add comment / NO: delete
```

### Investigation tools

| Source | Command |
|------|------|
| git blame | `git blame -L <line>,<line> <file>` |
| Commit messages | `git log --all -S "<code>" --oneline` |
| Issues/PRs | `gh search issues "<keyword>" --repo $(git remote get-url origin)` |
| ADR decisions | `.agents/memorys/decisions.md` |
| Community reference | `grep_app_searchGitHub` |

### Fence comments

When preserving complexity with reason, add a comment marker:

```rust
// chesterton: BufferPool uses unsafe zero-copy because GStreamer appsink outputs
// &[u8] that must be forwarded directly to WebRTC TrackLocal to avoid memcpy delay.
// Tried safe Vec<u8> copy approach → 1080p60 frame drop rate went from 0% → 1.2%.
// git: a1b2c3d "zero-copy buffer pool for 60fps stability"
unsafe {
    pool.copy_to_track(track, data);
}
```

## Rule of 500

| Check | Command | Threshold | Action |
|--------|------|------|------|
| File line count | `wc -l <file>` | >500 | Split into multiple domain modules |
| Function line count | `grep -n '^fn ' <file>`, estimate block | >50 | Extract helper functions |
| Parameter count | `grep -c ','` per fn sig | >5 | Merge parameters into a struct |
| Nesting depth | Manual indent check | >4 | Early return / `?` propagation |
| trait impl count | `grep -c 'impl.*for' <file>` | >3 | Split into impl submodules |
| match arm count | `grep -c '=>'` per match block | >10 | `enum_dispatch` / strategy pattern |
| pub exposure count | `grep -c 'pub' <file>` | >20 | Reduce visibility, finer-grained modules |

### Execution

```
# 1. Scan files exceeding limits
find crates/ -name '*.rs' -exec wc -l {} + | sort -rn | head -20

# 2. Apply Chesterton's Fence to each file over the limit
# 3. Split, extract, delete
# 4. Verify: cargo clippy -- -D warnings && cargo test -p <crate>
```

## TalkServo-specific patterns

### Borrow checker simplification

```rust
// BEFORE: unnecessary Arc<Mutex<>> nesting
let data: Arc<Mutex<Vec<Arc<Mutex<Option<Box<dyn Trait>>>>>>> = ...;

// AFTER: single ownership, borrow on demand
let mut data: Vec<Box<dyn Trait>> = Vec::new();
// When sharing is needed: let data = Arc::new(RefCell::new(data));
// ponytail: RefCell is fine for single-threaded use; switch to RwLock for real multi-threading
```

### WebRTC backend abstraction layer

```rust
// BEFORE: duplicated glue code in each backend
#[cfg(feature = "backend-webrtc-rs")]
fn create_pc() -> RTCPeerConnection { ... }
#[cfg(feature = "backend-webrtc-sys")]
fn create_pc() -> RTCPeerConnection { ... }
#[cfg(feature = "backend-stub")]
fn create_pc() -> RTCPeerConnection { ... }

// AFTER: extract commonality into shared module, backends only implement differences
// chesterton: the triple backend cfg duplication is an architectural cost of D15,
// cannot be eliminated but can be compressed into the backend/ submodule to minimize.
```

### Config path simplification

```rust
// BEFORE: deeply nested config
let port = config.server.webrtc.ice.transport.port.unwrap_or(9800);

// AFTER: flattened + sensible defaults
let port = env::var("OMSP_ICE_PORT").ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(9800);
```

## Relationship to ponytail

| ponytail | code-simplification |
|----------|---------------------|
| Whole-repo audit → sort | Single file/module deep-dive |
| "Can this be deleted?" | "Why does this exist?" |
| Deletion decisions | Split + refactor decisions |
| One finding per line | Structural refactoring |

**Workflow**: `/ponytail-audit` → sort → `/code-simplification` process each

## Verification gates

```bash
# 1. Compiles
cargo check --workspace --all-features

# 2. Clippy zero warnings
cargo clippy --workspace --all-features -- -D warnings

# 3. Tests pass
cargo test --workspace

# 4. File line count decreased
git diff --stat | grep -E '\+[0-9]+.*-' | tail -5

# 5. No new pub API added (simplification should not expand the public surface)
# ponytail: only check pub fn count, not enforced; refactoring may naturally add some
```

## Anti-pattern detection

| Anti-pattern | Detection command | Automated fix |
|--------|---------|-----------|
| `unwrap()` without comment | `grep -r 'unwrap()' crates/ --include='*.rs' \| grep -v '//.*unwrap'` | Replace with `?` or `.context()` |
| `clone()` to satisfy borrow | `grep -r '\.clone()' crates/ --include='*.rs'` | Analyze the borrow chain |
| `Box<dyn Trait>` could be generic | clippy `boxed_local` lint | Change to `impl Trait` |
| Feature-gated dead code | `cargo deadlinks` / `cargo udeps` | Remove the dead feature |
| Duplicate impl blocks | Manual inspection | Merge or extract macro |
| `as` type cast | `grep -r ' as ' crates/ --include='*.rs'` | Prefer `.into()` / `.try_into()` |

## Output format

```
## Simplification report

File: crates/talkservo-webrtc/src/backend/mod.rs
Before: 487 lines → after: 312 lines (-36%)

### Removed
- [lines 45-78] unused trait `LegacySdpParser` → deleted (git blame: introduced in D78, deprecated in D112)
- [lines 203-206] dead code `#[cfg(all(nonexistent, feature = "..."))]`

### Split
- [mod.rs] → `backend/webrtc_rs.rs`, `backend/webrtc_sys.rs`, `backend/shared.rs`

### Kept (Chesterton's Fence)
- [lines 180-195] `#[cfg]` triple backend duplication — D15 architectural cost, cannot be eliminated
- [lines 300] `unsafe` buffer pool — performance constraint, see comment

### Verification
✅ cargo clippy -- -D warnings
✅ cargo test -p talkservo-webrtc
```

## Forbidden

- Deleting code you don't understand
- Merging unrelated modules
- Simplifying `unsafe` blocks without understanding the memory semantics
- Removing "looks unused" code without checking git blame first
- Deleting feature-gated code without verifying the CI variant first
