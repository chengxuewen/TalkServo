#!/usr/bin/env bash
# scripts/gate.sh -- canonical ledger gates + link integrity (plan-8 P3).
#
# PIT-18 lesson, encoded structurally: ad-hoc retyped gate probes reported
# false results. The C1-C5 blocks below are embedded VERBATIM from
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

# ---- C4 (verbatim from conventions.md; do not edit here) ----
runb64 "C4" \
"c2V0IC11byBwaXBlZmFpbApkbWF4PSQoZ3JlcCAtbyAnXiMjIERbMC05XSonIC5hZ2VudHMvbWVt"\
"b3J5cy9kZWNpc2lvbnMubWQgfCBncmVwIC1vICdbMC05XSonIHwgc29ydCAtbiB8IHRhaWwgLTEp"\
"CnBtYXg9JChncmVwIC1vICdeIyMgUElULVswLTldKicgLmFnZW50cy9tZW1vcnlzL3BpdGZhbGxz"\
"Lm1kIHwgZ3JlcCAtbyAnWzAtOV0qJyB8IHNvcnQgLW4gfCB0YWlsIC0xKQpjbWF4PSQoZ3JlcCAt"\
"byAnXiMjIENbMC05XSonIC5hZ2VudHMvbWVtb3J5cy9jb252ZW50aW9ucy5tZCB8IGdyZXAgLW8g"\
"J1swLTldKicgfCBzb3J0IC1uIHwgdGFpbCAtMSkKaWRzPSQoZ3JlcCAtbyAnXiMjIERbMC05XSon"\
"IC5hZ2VudHMvbWVtb3J5cy9kZWNpc2lvbnMubWQgfCBzZWQgJ3MvXiMjIC8vJyB8IHNvcnQpCnJv"\
"d3M9JChncmVwIC1vRSAnXlx8IERbMC05XSsgXHwnIC5hZ2VudHMvbWVtb3J5cy9zdGF0dXMubWQg"\
"fCB0ciAtZCAnfCAnIHwgc29ydCkKZmFpbD0wCmdyZXAgLXEgIlJhbmdlczogRDEtRCRkbWF4LCBD"\
"MS1DJGNtYXgsIFBJVC0xLi4kcG1heFwuIiBBR0VOVFMubWQgXAogIHx8IHsgZWNobyAiRkFJTDog"\
"QUdFTlRTLm1kIFJhbmdlcyByb3cgc3RhbGUgKHdhbnQgRDEtRCRkbWF4LCBDMS1DJGNtYXgsIFBJ"\
"VC0xLi4kcG1heCkiOyBmYWlsPTE7IH0KZ3JlcCAtcSAicGl0ZmFsbHMubWQgUElULTEuLiRwbWF4"\
"ICIgQUdFTlRTLm1kIFwKICB8fCB7IGVjaG8gIkZBSUw6IEFHRU5UUy5tZCBXSEVSRS1UTy1MT09L"\
"IFBJVCByYW5nZSBzdGFsZSAod2FudCBQSVQtMS4uJHBtYXgpIjsgZmFpbD0xOyB9Cm1pc3Npbmc9"\
"JChjb21tIC0yMyA8KGVjaG8gIiRpZHMiKSA8KGVjaG8gIiRyb3dzIikpClsgLW4gIiRtaXNzaW5n"\
"IiBdICYmIHsgZWNobyAiRkFJTDogc3RhdHVzLm1kIERlY2lzaW9uIFN0YXR1cyB0YWJsZSBtaXNz"\
"aW5nIGlkczogJChlY2hvICRtaXNzaW5nIHwgdHIgJ1xuJyAnICcpIjsgZmFpbD0xOyB9Cmdob3N0"\
"cz0kKGNvbW0gLTEzIDwoZWNobyAiJGlkcyIpIDwoZWNobyAiJHJvd3MiKSkKWyAtbiAiJGdob3N0"\
"cyIgXSAmJiB7IGVjaG8gIkZBSUw6IHN0YXR1cy5tZCB0YWJsZSBjaXRlcyBpZHMgd2l0aCBubyBs"\
"ZWRnZXIgZW50cnk6ICQoZWNobyAkZ2hvc3RzIHwgdHIgJ1xuJyAnICcpIjsgZmFpbD0xOyB9Clsg"\
"IiRmYWlsIiAtZXEgMCBdICYmIGVjaG8gUEFTUw=="
# ---- END C4 ----

# ---- C5 (verbatim from conventions.md; do not edit here) ----
runb64 "C5" \
"b3V0PSQoZ3JlcCAtcm5FICdHaXRIdWIgQWN0aW9uc3xhY3Rpb25zL3J1bnN8bmlnaHRseXxDSSBj"\
"aGVjayB0YXJnZXR8Q0kgbGFuZXxDSVstIF1wYXJpdHknIFwKICBBR0VOVFMubWQgZG9jcy9hcmNo"\
"aXRlY3R1cmUubWQgZG9jcy9SRUFETUUubWQgZG9jcy9tb2R1bGVzIFwKICBkb2NzL3JlZmVyZW5j"\
"ZS9zZXJ2ZXItYXBpLm1kIGRvY3MvcmVmZXJlbmNlL2FkbWluLWV2ZW50cy5tZCBkb2NzL3JlZmVy"\
"ZW5jZS9SRUFETUUubWQgXAogIC5hZ2VudHMvc2tpbGxzIC5hZ2VudHMvcnVsZXMgLmFnZW50cy9t"\
"ZW1vcnlzL3N0YXR1cy5tZCAuYWdlbnRzL21lbW9yeXMvZGVjaXNpb25zLm1kIDI+L2Rldi9udWxs"\
"IFwKICB8IGdyZXAgLXZpRSAnaW5lcnR8RDI2JykKaWYgWyAtbiAiJG91dCIgXTsgdGhlbiBlY2hv"\
"ICJGQUlMOiBob3N0ZWQtdmVyaWZpY2F0aW9uIGNsYWltcyB3aXRoIG5vIHJ1bm5hYmxlIGNvbW1h"\
"bmQgYW5kIG5vIGluZXJ0IG1hcmtlcjoiOyBlY2hvICIkb3V0IjsgZXhpdCAxOyBmaQplY2hvIFBB"\
"U1M="
# ---- END C5 ----

# ---- C6 (verbatim from conventions.md; do not edit here) ----
runb64 "C6" \
"cHl0aG9uMyAtIDw8J1BZJwppbXBvcnQgcmUsIHN5cwpmcm9tIHBhdGhsaWIgaW1wb3J0IFBhdGgK"\
"CmRlZiBfbWF4X2lkKHBhdGgsIHBhdCk6CiAgICB0cnk6CiAgICAgICAgaWRzID0gcmUuZmluZGFs"\
"bChwYXQsIG9wZW4ocGF0aCwgZW5jb2Rpbmc9J3V0Zi04JykucmVhZCgpLCByZS5NKQogICAgZXhj"\
"ZXB0IE9TRXJyb3I6CiAgICAgICAgcmV0dXJuIDAKICAgIHJldHVybiBtYXgoKGludCh4KSBmb3Ig"\
"eCBpbiBpZHMpLCBkZWZhdWx0PTApCgpETUFYID0gX21heF9pZCgnLmFnZW50cy9tZW1vcnlzL2Rl"\
"Y2lzaW9ucy5tZCcsIHInXiMjIEQoXGQrKScpICAgICAgIyBsaXZlIGJvdW5kcyBmcm9tIHRoZSBs"\
"ZWRnZXJzCkNNQVggPSBfbWF4X2lkKCcuYWdlbnRzL21lbW9yeXMvY29udmVudGlvbnMubWQnLCBy"\
"J14jIyBDKFxkKyknKSAgICMgKEM0IG1ldGhvZDogZGVyaXZlZCwKUE1BWCA9IF9tYXhfaWQoJy5h"\
"Z2VudHMvbWVtb3J5cy9waXRmYWxscy5tZCcsIHInXiMjIFBJVC0oXGQrKScpICAjICBuZXZlciBy"\
"ZS1waW5uZWQgcGVyIGVudHJ5KQpwYXRocyA9IFtdCmZvciBwYXQgaW4gWyJkb2NzLyoqLyoubWQi"\
"LCAiLmFnZW50cy9ydWxlcy8qKi8qLm1kIiwgIi5hZ2VudHMvc2tpbGxzLyovU0tJTEwubWQiXToK"\
"ICAgIHBhdGhzICs9IFtwIGZvciBwIGluIFBhdGgoJy4nKS5nbG9iKHBhdCkKICAgICAgICAgICAg"\
"ICBpZiAncmVmZXJlbmNlL3Jlc2VhcmNoJyBub3QgaW4gc3RyKHApIGFuZCAnZG9jcy9wbGFucycg"\
"bm90IGluIHN0cihwKV0KcGF0aHMgKz0gW1BhdGgoJy5hZ2VudHMvbWVtb3J5cy9zdGF0dXMubWQn"\
"KSwgUGF0aCgnLmFnZW50cy9tZW1vcnlzL2RlY2lzaW9ucy5tZCcpXQpwYXRocyArPSBbUGF0aCgn"\
"LmFnZW50cy9BR0VOVFMubWQnKV0KcGF0aHMgKz0gW3AgZm9yIHAgaW4gUGF0aCgnY3JhdGVzJyku"\
"cmdsb2IoJyoucnMnKV0KCmJhZCA9IFtdCmZvciBwIGluIHBhdGhzOgogICAgZm9yIGksIGxpbmUg"\
"aW4gZW51bWVyYXRlKHAucmVhZF90ZXh0KCkuc3BsaXRsaW5lcygpLCAxKToKICAgICAgICBpZHMg"\
"PSByZS5maW5kYWxsKHInXGJEXGQrXGJ8XGJDXGQrXGJ8XGJQSVQtXGQrXGInLCBsaW5lKQogICAg"\
"ICAgIGtlZXAgPSBbXQogICAgICAgIGZvciB0b2sgaW4gaWRzOgogICAgICAgICAgICBpZiB0b2su"\
"c3RhcnRzd2l0aCgnUElULScpOgogICAgICAgICAgICAgICAgbiA9IGludCh0b2tbNDpdKQogICAg"\
"ICAgICAgICAgICAgaWYgbm90ICgxIDw9IG4gPD0gUE1BWCk6IGtlZXAuYXBwZW5kKHRvayk7IGNv"\
"bnRpbnVlCiAgICAgICAgICAgIGVsc2U6CiAgICAgICAgICAgICAgICBuID0gaW50KHRva1sxOl0p"\
"CiAgICAgICAgICAgICAgICBpZiB0b2suc3RhcnRzd2l0aCgnRCcpIGFuZCBub3QgKDEgPD0gbiA8"\
"PSBETUFYKToga2VlcC5hcHBlbmQodG9rKTsgY29udGludWUKICAgICAgICAgICAgICAgIGlmIHRv"\
"ay5zdGFydHN3aXRoKCdDJykgYW5kIG5vdCAoMSA8PSBuIDw9IENNQVgpOiBrZWVwLmFwcGVuZCh0"\
"b2spOyBjb250aW51ZQogICAgICAgIGlmIGtlZXAgYW5kIG5vdCByZS5zZWFyY2gocidNZWRpYVNl"\
"cnZvfHNpc3RlcnxpbmVydCcsIGxpbmUpOgogICAgICAgICAgICBiYWQuYXBwZW5kKGYne3B9Ontp"\
"fToge2tlZXB9IHtsaW5lLnN0cmlwKClbOjEwMF19JykKZm9yIGIgaW4gYmFkOiBwcmludCgnRkFJ"\
"TDonLCBiKQpyYWlzZSBTeXN0ZW1FeGl0KDEgaWYgYmFkIGVsc2UgMCkKUFk="
# ---- END C6 ----


# ---- POLYORCH (manual graph-floor gate) ----
echo "=== POLYORCH ==="
if bash scripts/gate-polyorch.sh; then echo "[PASS] POLYORCH"; else echo "[FAIL] POLYORCH"; rc=1; fi
# ---- END POLYORCH ----

# ---- LANES (manual lane registry gate) ----
echo "=== LANES ==="
if python3 scripts/gate-lanes.py --self-test && python3 scripts/gate-lanes.py; then echo "[PASS] LANES"; else echo "[FAIL] LANES"; rc=1; fi
# ---- END LANES ----

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
