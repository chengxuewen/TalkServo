# 3rdparty/oxmgr — vendored process manager (D19, plan-6)

OxMgr (Rust pm2-style supervisor; the mediaservo sister ships the same tool)
is the **runtime supervisor** for `talkservo-server` instance dirs
(`init/start/stop/restart/status/monit/logs/ps` facade). This directory =
**archives only**: upstream release `tar.gz + .sha256` kept verbatim (filename
= full coordinate). Extracted binaries live in gitignored `.cache/oxmgr/<archive>/`.

| item | x86_64 | aarch64 |
|---|---|---|
| upstream | github.com/Vladimir-Urik/OxMgr/releases/tag/v0.5.0 | same |
| verified | sha256sum -c OK 2026-10-08 | sha256sum -c OK 2026-10-08 |

**Consumption chain** (talkservo.sh `ensure_oxmgr`): pick archive by
`uname -m` → verify sha256 BEFORE unpack (mismatch = refuse, never silent)
→ extract to `.cache/oxmgr/<archive-name>/` (version in dir = bump
invalidates naturally) → PATH inject; deploy/package copies `bin/oxmgr`
into the instance (server facade resolves instance `bin/` first, then PATH).
Unknown arch (incl. macOS — no upstream darwin build) = explicit WARN, PATH
fallback, never a wrong-arch binary.

**Bump recipe**: download upstream tar.gz + sidecar checksum → drop both
here (keep upstream names) → bump `OXMGR_*` lines in `talkservo.sh` →
`rm -rf .cache/oxmgr` → any command re-extracts and smoke-starts.
