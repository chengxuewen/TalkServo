#!/usr/bin/env bash
# PoC token issuance (modules/06): HS256 JWT from TS_JWT_SECRET for dev/e2e.
# Usage: TS_JWT_SECRET=... scripts/issue-token.sh <sub> <room> <role:dispatch|field> [ttl_s] [mod_rooms_csv]
# No public signing endpoint exists; replaced by admin-issued tokens at Alpha.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_common.sh
source "$SCRIPT_DIR/_common.sh"

SUB="${1:?usage: issue-token.sh <sub> <room> <role> [ttl_s]}"
ROOM="${2:?room required}"
ROLE="${3:?role: dispatch|field}"
TTL="${4:-3600}"
SECRET="${TS_JWT_SECRET:?TS_JWT_SECRET must be set}"

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }

now="$(date +%s)"
exp=$((now + TTL))

header='{"alg":"HS256","typ":"JWT"}'
MODS="${5:-}"
mods_json=""
if [ -n "$MODS" ]; then
  # csv -> json array (names already charset-checked server-side on use)
  mods_json=",\"mod_rooms\":[\"${MODS//,/\",\"}\"]"
fi
claims="{\"sub\":\"$SUB\",\"room\":\"$ROOM\",\"role\":\"$ROLE\",\"aud\":\"$ROOM\"$mods_json,\"exp\":$exp}"

h64="$(printf '%s' "$header" | b64url)"
c64="$(printf '%s' "$claims" | b64url)"
sig="$(printf '%s.%s' "$h64" "$c64" | openssl dgst -sha256 -hmac "$SECRET" -binary | b64url)"

printf '%s.%s.%s\n' "$h64" "$c64" "$sig"
