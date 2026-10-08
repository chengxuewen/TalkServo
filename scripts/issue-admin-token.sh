#!/usr/bin/env bash
# Bootstrap admin-console token (plan-5, D18). HS256 over TS_ADMIN_SECRET —
# aud is FIXED to "admin" (AdminClaims carries no room; structural audience
# isolation). Run on the SERVER HOST: exp is compared against the server
# clock with zero leeway. No public signing endpoint exists; this script is
# the only admin-token mint (the console's Tokens page mints USER tokens).
# Usage: TS_ADMIN_SECRET=... scripts/issue-admin-token.sh <sub> [ttl_s]
set -euo pipefail
SUB="${1:?usage: issue-admin-token.sh <sub> [ttl_s]}"
TTL="${2:-3600}"
SECRET="${TS_ADMIN_SECRET:?TS_ADMIN_SECRET must be set}"
[ "$SECRET" = "${TS_JWT_SECRET:-}" ] && { echo "FATAL: admin secret must differ from TS_JWT_SECRET" >&2; exit 1; }

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }
now="$(date +%s)"
exp=$((now + TTL))
header='{"alg":"HS256","typ":"JWT"}'
claims="{\"sub\":\"$SUB\",\"aud\":\"admin\",\"exp\":$exp}"
h64="$(printf '%s' "$header" | b64url)"
c64="$(printf '%s' "$claims" | b64url)"
sig="$(printf '%s.%s' "$h64" "$c64" | openssl dgst -sha256 -hmac "$SECRET" -binary | b64url)"
printf '%s.%s.%s\n' "$h64" "$c64" "$sig"
