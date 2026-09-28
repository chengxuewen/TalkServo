---
name: test-harness
description: "TalkServo multi-language automated test harness. Generates test skeletons from SDD specs (Rust/TS/Python/C++/C), enforces AAA mode, test-to-spec reverse tracing, coverage reports. Interactive menu-driven. Supports Phase awareness (skips modules that are not ready)."
---

# Test Harness

> **Ledger note (doc-audit 2026-09-28, PIT-3)**: bare `PIT-{n}` / `C{n}` / `D{nn}` identifiers in this file cite the **MediaServo ledger** (sister project), not this repo — TalkServo's own ids live in `.agents/memorys/` (C1-C2, D1-D11, PIT-1-PIT-3).


Provides automated test generation and verification for the TalkServo multi-language project. Emits test code directly from SDD specs, ensuring AAA mode, Phase alignment, and per-language convention consistency.

**Philosophy**: Tests are not something added after the code is written — they grow directly out of the spec. A good test file = an executable copy of the spec.

---

## Entry: Test Task Types

### `/test-harness` (no arguments)
Opens the task type menu:

```
[1] SDD -> tests — generate test files from specs (stubs / AAA skeletons / full fill)
[2] Tests -> specs — reverse tracing (verify coverage / missing scenarios)
[3] Case execution — run tests and fix failing cases
[4] Incremental tests — produce incremental tests from the current git diff
[5] Project init — set up test infrastructure for a new module
[6] Coverage report — generate coverage analysis and gaps
[7] Selective testing — run only tests affected by changes (git diff driven)
```

### `/test-harness generate`
Skips the menu, goes straight into SDD -> test generation mode.

### `/test-harness run`
Skips the menu, runs tests directly.

### `/test-harness quick`
Only fix the currently failing tests, do not generate new tests.

---

## Multi-Language Strategy

TalkServo is a multi-language project (Rust core/server + TS web; sister-ledger D19/D21 lineage — this repo's scope lives in docs/modules/03). Test generation must adapt to each language's conventions.

### Language Detection

Detect target languages automatically from the project structure:

| Signal | Detected Language |
|------|---------|
| `Cargo.toml` exists | Rust |
| `package.json` + `tsconfig.json` | TypeScript |
| `pyproject.toml` / `setup.py` | Python |
| `CMakeLists.txt` + `.cpp`/`.hpp` | C++ |
| `CMakeLists.txt` + `.c`/`.h` (no .cpp) | C |
| Multiple signals | Interactive selection |

### Per-Language Testing Conventions

#### Rust
- **Unit tests**: `#[cfg(test)] mod tests { ... }` inlined in the source file
- **Integration tests**: `tests/` directory, each file an independent crate
- **Mocks**: `mockall` crate, `mock!` macro
- **Assertions**: `assert_eq!` / `assert!` / `assert!(matches!(...))`
- **Run**: `cargo test`
- **Conventions**:
  - Function naming: `test_<module>_<scenario>`
  - `unwrap()` / `expect()` forbidden — use `?` or `assert!(result.is_ok())`
  - Test module starts with `use super::*;`

#### TypeScript
- **Unit tests**: `*.test.ts` or `*.spec.ts` alongside the source file
- **Framework**: Vitest (preferred) / Jest
- **Mocks**: `vi.mock()` / `jest.mock()`
- **Assertions**: `expect(x).toBe(y)` / `expect(x).toEqual(y)`
- **Run**: `npx vitest` / `npx jest`
- **Conventions**:
  - `describe('ModuleName', () => { it('should ...', () => { ... }) })`
  - AAA comments: `// Arrange` / `// Act` / `// Assert`

#### Python
- **Unit tests**: `test_*.py` alongside the source file or under `tests/`
- **Framework**: pytest
- **Mocks**: `unittest.mock` / `pytest-mock`
- **Assertions**: plain `assert` (pytest style)
- **Run**: `pytest`
- **Conventions**:
  - Function naming: `test_<module>_<scenario>`
  - Class organization: `class Test<Module>:`
  - Fixture: `@pytest.fixture`

#### C++
- **Unit tests**: `*_test.cpp` under the `tests/` directory
- **Framework**: GoogleTest (gtest/gmock)
- **Mocks**: `MOCK_METHOD` macro
- **Assertions**: `EXPECT_EQ` / `ASSERT_TRUE`
- **Run**: `ctest` or `cmake --build build && ctest --test-dir build`
- **Conventions**:
  - `TEST(TestSuiteName, TestName) { ... }`
  - AAA comments

#### C
- **Unit tests**: `test_*.c` under the `tests/` directory
- **Framework**: Unity / CMock
- **Run**: `ctest` or `make test`
- **Conventions**:
  - `TEST_ASSERT_EQUAL(expected, actual)`
  - `setUp() / tearDown()` lifecycle

---

## Workflows

### Mode 1: SDD -> Test Generation

```
/test-harness generate
```

#### Generation Tiers (ask every time)

1. **stubs**: function signatures only + `todo!()` / `fail()` / `pytest.fail()` — compiles, tests fail
2. **AAA skeleton**: Arrange/Act/Assert comments + placeholders — structure ready, assertions to be filled
3. **Full fill**: concrete values extracted from the spec, complete runnable assertions — expected to pass directly

#### Step 1: Identify Spec Sources

Scan the `openspec/specs/` directory automatically:
```
openspec/specs/
├── hal-type-system-spec.md       -> 30 items (S-TYPE-*)
├── hal-qos-spec.md               -> 30 items (S-QOS-*)
├── config-barrier-spec.md        -> 24 items (S-CB-*)
└── hal-protocol-spec.md          -> 37 items (S-PROTO-*)
```

Let the user pick spec files (single or multiple selection).

#### Step 2: Phase-Aware Filtering

Read `docs/plans/p0-milestone-roadmap.md` and determine the current Phase:

| Phase | Available Specs |
|-------|---------|
| Phase 0 (CI) | Type system (S-TYPE) — pure logic, no trait dependencies |
| Phase 1 (hal-core) | S-TYPE + S-QOS + S-CB + S-PROTO — traits ready |
| Phase 2+ | All |

Filter spec items automatically:
- Spec items not ready in the current Phase -> mark ⏭️ skipped, generate an explanatory comment
- Priority A (P0) -> generate first, full fill
- Priority B/B+/C -> stubs or AAA skeleton

#### Step 3: Generate Test Files

For each spec item, generate a test function in the corresponding language:

```
Input: S-TYPE-001
  - Preconditions: Bool = true / false
  - Action: encode -> decode
  - Expected: true <-> true, false <-> false
  - Test mapping: test_type_01_bool_roundtrip

Output (Rust):
  #[test]
  fn test_type_01_bool_roundtrip() {
      // Arrange
      let values = vec![true, false];
      // Act
      for val in values {
          let encoded = encode_bool(val);
          let decoded = decode_bool(&encoded);
          // Assert
          assert_eq!(val, decoded, "Bool roundtrip failed");
      }
  }
```

#### Step 4: Write and Verify

1. Write the test files (inline unit tests / new integration test files)
2. Run a compile check (no syntax errors)
3. Run the tests (expected: some pass, some fail -> mark as to-be-implemented)
4. Report: `generated N test functions -> M passed / K failed / P skipped`

---

### Mode 2: Tests -> Spec Reverse Tracing

```
/test-harness trace
```

Check existing test coverage and cross-reference against the SDD specs:

1. Scan all test files
2. Extract test function names -> map to spec IDs (e.g. `test_type_01_*` -> S-TYPE-001)
3. Produce a coverage matrix:

```
Spec ID     | Test function                         | Status
-----------|------------------------------------|------
S-TYPE-001 | test_type_01_bool_roundtrip        | ✅
S-TYPE-002 | test_type_02_s8_roundtrip          | ✅
S-TYPE-003 | —                                  | ❌ missing
S-CB-007   | test_cb_07_partial_failure          | ⚠️ incomplete
```

4. Mark:
   - ❌ missing -> recommend generating from SDD
   - ⚠️ incomplete -> boundary conditions not covered
   - ✅ complete

---

### Mode 3: Case Execution & Repair

```
/test-harness run
```

1. Run the current project's test suite
2. Collect the failure list
3. For each failure:
   - Read the test source
   - Read the corresponding implementation source
   - Determine root cause: test error vs implementation error
   - Auto-fix test errors (wrong assertions, missing mocks)
   - Mark implementation errors -> report to the user

**Root-cause determination rules**:
- Test logic inconsistent with the spec -> test error
- Wrong assertion value (expects 1 but spec says 2) -> test error
- Implementation incomplete / interface changed -> implementation error

**Never**: modify implementation code to make tests pass (unless the user explicitly asks).

---

### Mode 4: Incremental Tests

```
/test-harness incremental
```

Identify changes from `git diff` -> generate the corresponding tests:

1. `git diff --name-only` to get the changed files
2. Reverse-map to SDD specs (file path -> module -> spec ID)
3. Generate incremental tests only for spec items related to the changes
4. If a changed file has no tests yet -> initialize a test file
(Note: Mode 7 runs affected tests; Mode 4 generates incremental tests)

---

### Mode 5: Project Initialization

```
/test-harness init
```

Create test infrastructure for a new module:

**Rust**:
- `tests/` directory + integration test entry points
- crate-level `#[cfg(test)]` helper module
- `mockall` dependency check

**TypeScript**:
- `vitest.config.ts` / `jest.config.ts`
- `__tests__/` directory
- test setup file

**Python**:
- `tests/__init__.py` + `conftest.py`
- pytest configuration (pyproject.toml)

**C++**:
- `CMakeLists.txt` test targets
- `tests/` directory + CMakeLists.txt
- gtest integration

**C**:
- `CMakeLists.txt` test targets
- Unity/CMock framework download

---

### Mode 6: Coverage Report

```
/test-harness coverage
```

1. Run the tests with coverage
2. Aggregate and display per module:

```
Module               | Line cov  | Branch cov | Spec coverage
-------------------|----------|----------|----------
hal-type-system    | 92%      | 85%      | 28/30
hal-qos            | 78%      | 71%      | 22/30
config-barrier     | 65%      | 58%      | 18/24
hal-protocol       | 88%      | 82%      | 34/37
───────────────────|──────────|──────────|────────
Total              | 81%      | 74%      | 102/121
```

3. Gap ranking: uncovered spec items / functions with low branch coverage
4. Recommendation: the top N tests to add first

---

### Mode 7: Selective Test Runner

```
/test-harness selective
```

> Note: talkservo and hal-flatbuffers are Phase 1 stub crates (M0.3) with no real test targets yet. Mode 7 uses `cargo metadata` to check crate existence before suggesting test commands.
Identify changes from `git diff` -> run only the affected tests (not a gate, a scheduling tool):

1. `git diff --name-only HEAD` to get the changed files
2. Map files to test targets:

| Changed path match | Test command |
|---|---|
| `crates/talkservo-common/` | `cargo test -p talkservo-common` |
| `crates/talkservo/` | `cargo test -p talkservo` (Phase 1, stub only) |
| `crates/hal-flatbuffers/` | `cargo test -p hal-flatbuffers` (Phase 1, stub only) |
| `*.fbs` (FlatBuffers schema) | `cargo test -p hal-flatbuffers` |
| `Cargo.toml` or `Cargo.lock` | `cargo test --workspace` |
| Cross-crate (3+ crates) | `./scripts/qa-fast.sh` |

3. Run the tests (one layer at a time, avoid cargo lock contention)
4. Report: layer / command / test count / passed / failed / duration

**Never**: run `cargo test` for multiple crates at the same time. Do not diagnose failures — report only. More than 20 changed files -> recommend `cargo test --workspace`.

---

## TalkServo-Specific Test Patterns

### HAL Trait Tests

All HAL core trait tests use `MockHalTransport` (`talkservo`):

```rust
// Rust pattern
#[test]
fn test_signal_write_read() {
    // Arrange
    let mut transport = MockHalTransport::new();
    let signal = Signal::new("test.value", HalValue::S32(42));

    // Act
    transport.write_signal(&signal).unwrap();
    let result = transport.read_signal("test.value").unwrap();

    // Assert
    assert_eq!(result, HalValue::S32(42));
}
```

### HalQoS Security Domain Tags

```rust
#[test]
fn test_qos_security_domain_hierarchical() {
    let tag = SecurityDomain::parse("l1.control.reactor_a").unwrap();
    assert!(tag.matches("l1.*"));
    assert!(tag.matches("l1.control.*"));
    assert!(!tag.matches("l2.*"));
}
```

### Config Barrier State Machine

```rust
#[test]
fn test_config_barrier_state_machine() {
    let mut barrier = ConfigBarrier::new();
    assert_eq!(barrier.state(), BarrierState::Idle);

    // Queue config change
    barrier.queue(ConfigChange::UpdateSignal { ... });
    assert_eq!(barrier.state(), BarrierState::Pending);

    // Apply at cycle boundary
    barrier.apply().unwrap();
    assert_eq!(barrier.state(), BarrierState::Idle);
}
```

### FlatBuffers Round-Trip

```rust
#[test]
fn test_halvalue_fbs_roundtrip() {
    let original = HalValue::S32(42);
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let offset = original.serialize(&mut builder);
    builder.finish(offset, None);
    let buf = builder.finished_data();
    let restored = HalValue::deserialize(buf).unwrap();
    assert_eq!(original, restored);
}
```

---

## Interactive Mode

All test-generation operations show a diff preview before writing files, confirmed by the user:

```
Will generate the following changes:
  crates/talkservo-common/src/types.rs +45 (inline test module)
  tests/integration/test_type_roundtrip.rs (new file, 150 lines)
  tests/integration/test_qos_security.rs (new file, 80 lines)

Total: 3 files, +275 lines, 30 test functions

Execute? [Y/n]
```

---

## Phase-Aware Rules

Read `docs/plans/p0-milestone-roadmap.md` automatically to confirm the current milestone.

| Checkpoint | Behavior |
|--------|------|
| Trait required by the spec not yet defined | Generate a stub or skip, annotate "⏭️ Phase 2" |
| Priority A (P0 must-have) | Full fill + assertions |
| Priority B/B+ | AAA skeleton |
| Priority C | Stub placeholder |

When the Phase changes, re-run `generate` -> previously skipped tests get filled in automatically.

---

## Quality Rules

1. **One assertion target per test** — one test function verifies one spec item
2. **AAA comments must be explicit** — `// Arrange` / `// Act` / `// Assert` may not be omitted
3. **Never modify tests to make the implementation pass** — precedence: spec > test > implementation
4. **No unwrap/expect** — avoid them in tests too; use `assert!(result.is_ok())`
5. **Boundary conditions first** — every entry of the spec's `boundary conditions` field -> its own test
6. **Traceable naming** — test function names include the spec ID (e.g. `test_type_01_*`)
7. **Phase alignment** — do not generate tests that the current Phase cannot run

---

## Quick Reference

```
/test-harness               -> choose task type
/test-harness generate      -> SDD -> test generation
/test-harness run           -> run and fix tests
/test-harness trace         -> tests -> spec reverse tracing
/test-harness incremental   -> incremental tests (git diff)
/test-harness init          -> initialize test infrastructure
/test-harness coverage      -> coverage report
/test-harness selective    -> selective test execution (git diff)
```

### Language Command Cheat Sheet

| Language | Run Tests | Coverage | Mock Library |
|------|---------|--------|---------|
| Rust | `cargo test` | `cargo tarpaulin` | mockall |
| TS | `npx vitest` | `npx vitest --coverage` | vi.mock() |
| Python | `pytest` | `pytest --cov` | pytest-mock |
| C++ | `ctest` | `gcov + lcov` | gmock |
| C | `ctest` | `gcov + lcov` | CMock |
