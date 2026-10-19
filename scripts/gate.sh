#!/usr/bin/env bash
# scripts/gate.sh -- canonical ledger gates + link integrity (plan-8 P3).
#
# PIT-18 lesson, encoded structurally: ad-hoc retyped gate probes reported
# false results. The C1-C3 blocks below are embedded VERBATIM from
# .agents/memorys/conventions.md (base64, decoded at run time -- zero quoting
# drift). Refresh ONLY via: python3 scripts/gate-gen.py
#
# Each block runs in an isolated bash -c child: canonical bodies may use
# exit 1 without poisoning the runner.
set -uo pipefail
cd "$(dirname "$0")/.."
rc=0
runb64() {  # runb64 <label> <base64-of-script>
    local label="$1" b64="$2" script
    script="$(printf '%s' "$b64" | base64 -d)"
    echo "=== $label ==="
    if bash -c "$script"; then echo "[PASS] $label"; else
        echo "[FAIL] $label"; rc=1
    fi
}

# ---- C1 (verbatim from conventions.md; do not edit here) ----
runb64 "C1" \
"b3V0PSQoZ3JlcCAtcm5QICdbXHh7NGUwMH0tXHh7OWZmZn1dJyAtLWV4Y2x1ZGUtZGlyPW5vZGVf"\
"bW9kdWxlcyAtLWV4Y2x1ZGUtZGlyPS5naXQgLS1leGNsdWRlLWRpcj10YXJnZXQgXAogIC0tZXhj"\
"bHVkZS1kaXI9X19weWNhY2hlX18gLS1leGNsdWRlLWRpcj1kaXN0IC0tZXhjbHVkZS1kaXI9Y292"\
"ZXJhZ2UgXAogIC5hZ2VudHMgLm9wZW5jb2RlIC5vbW8gY3JhdGVzIGFkbWluLWRhc2hib2FyZCBk"\
"b2NzIHdlYiAyPi9kZXYvbnVsbCB8IGdyZXAgLXYgJ2MxOmFsbG93LXpoJykKaWYgWyAtbiAiJG91"\
"dCIgXTsgdGhlbiBlY2hvICJGQUlMOiBub24tRW5nbGlzaCBhcnRpZmFjdHM6IjsgZWNobyAiJG91"\
"dCI7IGV4aXQgMTsgZmkKZWNobyBQQVNT"
# ---- END C1 ----

# ---- C2 (verbatim from conventions.md; do not edit here) ----
runb64 "C2" \
"dGVzdCAtZiBkb2NzL3JlZmVyZW5jZS9SRUFETUUubWQgJiYgdGVzdCAtZCBkb2NzL21vZHVsZXMg"\
"JiYgISBscyBkb2NzL3Jlc2VhcmNoIDI+L2Rldi9udWxsICYmIGVjaG8gUEFTUw=="
# ---- END C2 ----

# ---- C3 (verbatim from conventions.md; do not edit here) ----
runb64 "C3" \
"b3V0PSQoZ3JlcCAtcm5FICdwYXRoPSIvKGF8ZHxmKSgvfCIpfCIvKGF8ZHxmKS8iJyB3ZWIvc3Jj"\
"IHdlYi9lMmUgY3JhdGVzL3RhbGtzZXJ2by1zZXJ2ZXIvc3JjIDI+L2Rldi9udWxsKQppZiBbIC1u"\
"ICIkb3V0IiBdOyB0aGVuIGVjaG8gIkZBSUw6IHNpbmdsZS1sZXR0ZXIgcm91dGUgcGF0aHM6Ijsg"\
"ZWNobyAiJG91dCI7IGV4aXQgMTsgZmkKZWNobyBQQVNT"
# ---- END C3 ----

# ---- LINK (manual region: user rows live outside managed blocks) ----
echo "=== LINK ==="
if python3 -c '
import re,os,glob
bad=[p+" -> "+m for p in glob.glob("docs/**/*.md",recursive=True)
     for m in re.findall(r"\]\(([^)h][^)]*\.md)",open(p).read())
     if not os.path.exists(os.path.normpath(os.path.join(os.path.dirname(p),m)))]
print("\n".join(bad) or "PASS"); raise SystemExit(1 if bad else 0)
'; then echo "[PASS] LINK"; else echo "[FAIL] LINK"; rc=1; fi
# ---- END LINK ----

exit $rc
