---
name: security-hardening
description: "TalkServo security audit and hardening. OWASP Top 10 checks, hardcoded secrets/ports/URLs scan (merged review-hardcode), PSK/JWT auth flow review, WebSocket security, secrets management (MediaServo PIT-10 lesson), mediasoup SFU transport security. Use before release, after auth changes, or when onboarding new PSK keys. Also accessible via /review-hardcode."
---

# security-hardening — Security Hardening

> **Ledger note (doc-audit 2026-09-28, PIT-3)**: bare `PIT-{n}` / `C{n}` / `D{nn}` identifiers in this file cite the **MediaServo ledger** (sister project), not this repo — TalkServo's own ids live in `.agents/memorys/` (C1-C2, D1-D11, PIT-1-PIT-3).


> OWASP Top 10 + PSK/JWT auth + WebSocket security + secrets management.
> Every rule has a check command. Every check must pass.

## Trigger Conditions

- Release candidate
- Auth module modified
- New WebSocket endpoint added
- New PSK key / JWT secret added
- User says "security review" / "安全审计"  <!-- c1:allow-zh -->
- New FFI boundary (C/Objective-C bridge)

## Mode A: Audit Mode

Full security audit, manually triggered. Runs all 6 Phases, produces an audit report.

## Mode B: Guard-while-building (PreToolUse)

> Triggers automatically on code changes, blocks insecure commits. Lightweight, target <5s.
> Reference: JoeyPatricio/security-hardening-skill — PreToolUse guard pattern.

### B1: cargo-audit quick scan

```bash
# Check only newly introduced vulnerabilities (not full audit)
cargo audit --quiet --deny unsound 2>&1 | head -20
```

| Check item | Command | Pass criteria |
|--------|------|---------|
| Known vulnerabilities | `cargo audit --quiet` | 0 RUSTSEC warnings |
| Deny unsound | `cargo audit --deny unsound` | exit 0 |

### B2: unsafe block scan

```bash
# Every unsafe block must have a // SAFETY: comment
UNSAFE_NO_COMMENT=$(grep -rn 'unsafe' crates/ --include='*.rs' | grep -v '// SAFETY:' || true)
if [ -n "$UNSAFE_NO_COMMENT" ]; then
  echo "ERROR: unsafe block without SAFETY comment:"
  echo "$UNSAFE_NO_COMMENT"
  exit 1
fi
```

### B3: gitleaks secret detection

```bash
# Install (one-time)
# brew install gitleaks  # macOS
# or: go install github.com/gitleaks/gitleaks/v8@latest

# PreToolUse check: scan staged changes
gitleaks detect --source . --no-git --redact --verbose 2>&1 | head -30
```

### B4: Hardcoded quick scan (lightweight)

```bash
# Scan only changed files (not the whole repo)
git diff --cached --name-only -- '*.rs' | xargs -I{} grep -n 'token.*=\|password.*=\|api[_-]key' {} 2>/dev/null || true
```

### Guard execution order

```text
PreToolUse (on edit .rs/.toml):
  1. unsafe block comment check   (<0.5s)
  2. hardcoded quick scan          (<1s)
  3. gitleaks detection            (<2s)
  4. cargo audit                   (<3s)

Any failure → block the operation
```

### gitleaks configuration (.gitleaks.toml)

```toml
# .gitleaks.toml — project-level config
title = "TalkServo gitleaks config"

[extend]
useDefault = true

[allowlist]
description = "Known false positives"
paths = [
  "scripts/scan-hardcode.sh",  # test patterns
  ".env.example",               # placeholder values
]

[[rules]]
id = "custom-psk-pattern"
description = "TalkServo PSK keys"
regex = '''(?i)(psk|pre_shared_key)\s*=\s*["'][A-Za-z0-9+/]{32,}["']'''
```

### gitleaks pre-commit hook (.git/hooks/pre-commit)

> MediaServo reference D201: their pre-commit hook (not yet installed here) runs `cargo fmt --check` + `cargo clippy -- -D warnings`.
> Below is the gitleaks integration extending the same hook:

```bash
#!/usr/bin/env bash
# .git/hooks/pre-commit — Rust quality gate + gitleaks secret detection
set -euo pipefail

STAGED_RS=$(git diff --cached --name-only --diff-filter=ACM -- '*.rs' || true)
STAGED_TOML=$(git diff --cached --name-only --diff-filter=ACM -- '*.toml' || true)

# ---- gitleaks secret detection (full repo, only when .rs/.toml changed) ----
if [ -n "$STAGED_RS" ] || [ -n "$STAGED_TOML" ]; then
  if command -v gitleaks &>/dev/null; then
    echo "→ gitleaks: scanning staged changes..."
    { gitleaks detect --source . --no-git --redact --verbose 2>&1; } || {
      echo ""
      echo "ERROR: gitleaks detected secrets in staged files."
      echo "  Review findings above. False positive? Add to .gitleaks.toml allowlist."
      echo "  To bypass (emergency only): GITLEAKS_SKIP=1 git commit ..."
      exit 1
    }
  else
    echo "⚠ gitleaks not installed. Install: brew install gitleaks"
  fi
fi

# ---- Rust quality gate (only when .rs changed) ----
if [ -n "$STAGED_RS" ]; then
  echo "→ cargo fmt: checking..."
  cargo fmt --check
  echo "→ cargo clippy: checking..."
  cargo clippy -- -D warnings
fi

echo "✅ pre-commit checks passed"
```

### gitleaks installation

| Platform | Command |
|------|------|
| macOS | `brew install gitleaks` |
| Linux | `go install github.com/gitleaks/gitleaks/v8@latest` or download the [release binary](https://github.com/gitleaks/gitleaks/releases) |
| Docker | `docker run -v $(pwd):/path zricethezav/gitleaks detect --source /path` |

## Phase 1: Secret Scan (PIT-10 enforced)

```bash
# Run the hardcoded scan
./scripts/scan-hardcode.sh

# Manual supplementary checks
grep -rn 'api[_-]key\|api[_-]secret\|token.*=\|password.*="' crates/ --include='*.rs' | grep -v '//.*TODO' | grep -v 'env::var'
grep -rn 'sk-\|pk-\|AKID\|SecretId' crates/ --include='*.rs'
grep -rn 'apiKey\|api_key\|API_KEY' .opencode/ --include='*.json' --include='*.jsonc'
```

### PIT-10 rule

> Hardcoded API keys in global configuration are a leak risk. Use environment variable interpolation: `"apiKey": "{env:NEW_API_KEY}"`.

| Check item | Command | Pass criteria |
|--------|------|---------|
| No hardcoded secrets | `scripts/scan-hardcode.sh` | 0 CRITICAL |
| env var interpolation | `grep -rn '{env:' .opencode/` | All apiKey use interpolation |
| .gitignore coverage | `grep '\.env' .gitignore` | Contains .env, .env.local |
| Example file placeholder | `grep 'EXAMPLE_KEY\|your-key-here' .env.example` | No real secrets |

### Hardcoded value severity (formerly review-hardcode)

This skill absorbs the full scan capability of `review-hardcode`. The `/review-hardcode` command points here.

| Pattern | Severity | Explanation |
|------|--------|------|
| `token="..."` / `password="..."` / `secret="..."` / `api_key="..."` | 🔴 CRITICAL | Hardcoded secret/token |
| `:9800` etc. hardcoded ports | 🟠 HIGH | Production ports should not be hardcoded |
| `localhost:PORT` / `127.0.0.1:PORT` | 🟠 HIGH | Address+port should be configurable |
| `http://IP` hardcoded IP URL | 🟡 MEDIUM | Should use config or DNS |

### Exclusions

The scan automatically excludes: `target/`, `node_modules/`, `.git/`, `.pixi-cache/`.
Hardcoded values marked `TODO:` (allowed to exist temporarily) should be manually reviewed and then accepted or rejected.

## Phase 2: Auth Flow Audit

### PSK authentication (Host↔Server)

```rust
// TalkServo PSK flow:
// Host ──[PSK in WS header]──> Server ──[validate]──> Session token

// Checkpoints:
// 1. Is the PSK injected via environment variable? (not plaintext in config file)
// 2. Is the PSK ≥ 32 bytes?
// 3. Does the Server rate-limit PSK validation? (brute-force protection)
// 4. Does the session token have an expiration time?
// 5. Is the PSK error response vague? (do not leak "user exists" or "key is close")
```

```bash
# Check commands
grep -rn 'pre_shared_key\|psk' crates/talkservo-server/src/ --include='*.rs'
grep -rn 'env::var.*PSK\|env::var.*SECRET' crates/ --include='*.rs'
grep -rn 'session.*ttl\|token.*expir\|jwt.*exp' crates/talkservo-common/src/auth/ --include='*.rs'
grep -rn 'rate.limit\|429\|too.many' crates/talkservo-server/src/ --include='*.rs'
```

### JWT authentication (Admin UI)

```bash
# Check commands
# 1. Is the JWT secret ≥ 256 bits?
grep -rn 'jwt.*secret\|JWT_SECRET' crates/ --include='*.rs' | grep -v env::var

# 2. Confirm the algorithm is HS256 or RS256 (not none)
grep -rn 'Algorithm\|alg.*HS\|alg.*RS' crates/talkservo-common/src/auth/ --include='*.rs'

# 3. Confirm an exp claim exists
grep -rn 'exp\|expir' crates/talkservo-common/src/auth/ --include='*.rs'

# 4. Confirm refresh token rotation
grep -rn 'refresh_token\|refresh' crates/talkservo-server/src/ --include='*.rs'
```

## Phase 3: WebSocket Security

```bash
# Check commands
# 1. Message size limit (OOM protection)
grep -rn 'max.*message\|max.*frame\|message.*size' crates/talkservo-server/src/ --include='*.rs'

# 2. Connection rate limit
grep -rn 'connection.*limit\|max_connections\|concurrent' crates/talkservo-server/src/ --include='*.rs'

# 3. Origin validation
grep -rn 'origin\|allowed_origin\|verify_origin' crates/talkservo-server/src/ --include='*.rs'

# 4. TLS (production environment)
grep -rn 'wss://\|tls\|ssl_config' crates/talkservo-server/src/ --include='*.rs'
```

### Message injection protection

```rust
// All WS message deserialization must use strict serde mode
// Forbidden: serde_json::from_str(&raw) — unvalidated extra fields
// Correct: serde_json::from_str::<StrictSchema>(&raw) or deny_unknown_fields
```

```bash
grep -rn '#[serde(deny_unknown_fields)\|unknown_fields\|additional_properties' crates/ --include='*.rs'
grep -rn 'serde_json::from_slice\|serde_json::from_str' crates/talkservo-server/src/ --include='*.rs'
```

## Phase 4: mediasoup SFU Security

```bash
# Check commands
# 1. Is the RTP port range controlled (not 0-65535)?
grep -rn 'rtc_min_port\|rtc_max_port\|rtp_port' crates/talkservo-server/src/ --include='*.rs'

# 2. Does the WebRTC transport require auth?
grep -rn 'web_rtc_server\|webRtcTransport\|transport.*auth' crates/talkservo-server/src/ --include='*.rs'

# 3. Does Room creation require permission?
grep -rn 'create_room\|router.*create' crates/talkservo-server/src/sfu/ --include='*.rs'

# 4. Producer/Consumer permission isolation
grep -rn 'peer_id\|producer.*peer\|consumer.*peer' crates/talkservo-server/src/sfu/ --include='*.rs'
```

## Phase 5: Dependency Audit

```bash
# Run cargo-deny to check known vulnerabilities
cargo deny check advisories

# Run cargo-audit
cargo audit

# Check unsafe usage
grep -rn 'unsafe' crates/ --include='*.rs' | grep -v '// SAFETY:'
# Rule: every unsafe block must have a // SAFETY: comment explaining why it is safe
```

## Phase 6: Admin UI Security

```bash
# Check commands
# 1. Does the Dashboard require authentication?
grep -rn 'auth\|login\|redirect.*login' crates/talkservo-server/src/admin/ --include='*.rs'

# 2. Is there CSRF protection?
grep -rn 'csrf\|xsrf\|same_site' crates/talkservo-server/src/ --include='*.rs'

# 3. Is CORS strict?
grep -rn 'access-control\|allow_origin\|cors' crates/talkservo-server/src/ --include='*.rs'

# 4. Content Security Policy
grep -rn 'content-security\|CSP\|frame-ancestors' crates/talkservo-server/src/admin/ --include='*.rs'
```

## Security Checklist (OWASP Top 10 aligned)

| # | Check item | TalkServo counterpart | Command | Required |
|---|--------|-------------|------|:---:|
| A01 | Broken access control | Auth trait fully implemented | `grep -rn 'TODO\|FIXME' crates/*/src/auth/` | ✅ |
| A02 | Cryptographic failure | PSK ≥32B, TLS wss:// | `grep -rn 'wss://' crates/` (production check) | ✅ |
| A03 | Injection | WS message strict deser | `grep -rn 'deny_unknown' crates/` | ✅ |
| A04 | Insecure design | Rate limiting + timeouts | `grep -rn 'rate.limit\|timeout' crates/` | ✅ |
| A05 | Security misconfiguration | No debug mode in production | `grep -rn 'debug_assert\|cfg(debug)' crates/` | ✅ |
| A06 | Vulnerable components | cargo-audit passes | `cargo audit` | ✅ |
| A07 | Authentication failures | PSK + JWT dual mode | All Phase 2 checks | ✅ |
| A08 | Software and data integrity | FlatBuffers validation | (Phase 2+) | ✅ |
| A09 | Logging and monitoring failure | Audit logs | `grep -rn 'audit\|security.log' crates/` | ⚠️ |
| A10 | SSRF | No server-side HTTP fetches | `grep -rn 'reqwest\|hyper::Client' crates/talkservo-server/` | ✅ |

## Report Format

```
## Security Audit Report — [Date]

### Phase 1: Secret Scan
✅ Scan passed: 0 CRITICAL, 0 HIGH, 0 MEDIUM

### Phase 2: Auth Flow
✅ PSK from environment variable (OMSP_PSK)
✅ PSK length: 64 bytes
⚠️ Session TTL: 24h (2h recommended)
❌ No rate limiting (P0)

### Phase 3: WebSocket
✅ Message size limit: 1MB
❌ No Origin validation (CSWSH attackable)
⚠️ TLS only enabled in Docker environment

### Phase 4: mediasoup
✅ RTP ports: 40000-40100
✅ Room requires auth token
⚠️ Producer has no bandwidth limit

### Phase 5: Dependencies
✅ cargo-audit: 0 vulnerabilities
✅ cargo-deny: 0 unlicensed

### Summary
CRITICAL: 0 | HIGH: 1 | MEDIUM: 2
Fix suggestions: [sorted by severity]
```

## Forbidden

- Skip the PIT-10 anti-pattern: never hardcode secrets
- Silently swallow errors: auth failures must be logged without leaking secrets
- Ignore cargo-audit warnings: every RUSTSEC must have a decision record
- Use debug configuration in production: debug_assert! does not run in release
- Plaintext WS over HTTP: production must use wss:// or a controlled internal network
