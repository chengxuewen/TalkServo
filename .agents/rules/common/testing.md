# Testing Requirements

## Minimum Test Coverage: 80%

Test Types (ALL required):
1. **Unit Tests** - Individual functions, utilities, components
2. **Integration Tests** - API endpoints, database operations
3. **E2E Tests** - Critical user flows (framework chosen per language)

## Test-Driven Development

MANDATORY workflow:
1. Write test first (RED)
2. Run test - it should FAIL
3. Write minimal implementation (GREEN)
4. Run test - it should PASS
5. Refactor (IMPROVE)
6. Verify coverage (80%+)

## Troubleshooting Test Failures

1. Use **tdd-guide** agent
2. Check test isolation
3. Verify mocks are correct
4. Fix implementation, not tests (unless tests are wrong)

## Agent Support

- **tdd-guide** - Use PROACTIVELY for new features, enforces write-tests-first

## Test Structure (AAA Pattern)

Prefer Arrange-Act-Assert structure for tests:

```typescript
test('calculates similarity correctly', () => {
  // Arrange
  const vector1 = [1, 0, 0]
  const vector2 = [0, 1, 0]

  // Act
  const similarity = calculateCosineSimilarity(vector1, vector2)

  // Assert
  expect(similarity).toBe(0)
})
```

### Test Naming

Use descriptive names that explain the behavior under test:

```typescript
test('returns empty array when no markets match query', () => {})
test('throws error when API key is missing', () => {})
test('falls back to substring search when Redis is unavailable', () => {})
```

## Enforceable Checks

```bash
# Verify: tests pass
cargo test --workspace
# Verify: coverage
cargo tarpaulin --workspace 2>/dev/null || echo "tarpaulin not installed"
```

## Browser-Lane Coverage Rules (this project, learned 2026-10-21)

### 1. A control whose job is a network call is tested on the CALL, not on the control

Asserting a button/switch exists proves rendering, not wiring — that is how the
registry create flow shipped broken (`PIT-28`: typed values never reached the POST).
The case must assert method + path + body.

```bash
# ratchet: outbound-body assertions in the admin suite (baseline 2, both from 2026-10-21:
# the kick DELETE body and the rooms-create POST body)
n=$(grep -c 'method === "POST"\|method === "DELETE"' web/tests/admin.test.tsx)
[ "$n" -ge 2 ] || { echo "FAIL: outbound-body assertions fell to $n (baseline 2)"; exit 1; }
echo "PASS: $n outbound-body assertions"
```
Adding a write path to `web/src/lib/adminApi.ts` must raise that baseline in the same
commit — an un-raised number means the new call is covered by nothing but a click.

Honest coverage statement as of 2026-10-21, so nobody reads the ratchet as full cover:
body-asserted = `kickPeer` (vitest + e2e), `createRoom` (vitest + e2e). Asserted only at
the API/Rust layer = `deleteRoom`, `forceRelease`, `createAccount`, `removeAccount`,
`login` (the last has a vitest case that asserts the stored token, not the request body).
`login` and `audit` are exercised end-to-end by the Rust lanes; the UI-only paths above are
the ones a payload regression would slip through.

### 2. An env-gated spec needs a lane that sets the env var

`admin.spec.ts` self-skips without `TS_ADMIN` (the server only mints the admin secret when
asked). A lane that runs it unset reports **green while executing nothing** — the exact false
evidence this project rejects (PIT-1, PIT-27).

```bash
grep -q "TS_ADMIN=1" cmake/TalkServoGates.cmake \
  || { echo "FAIL: e2e-stub no longer sets TS_ADMIN=1; the admin spec would self-skip"; exit 1; }
```
