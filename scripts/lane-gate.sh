#!/usr/bin/env bash
# lane-gate.sh — run a command only when an environment variable matches, otherwise exit 77.
#
# 77 is ctest's SKIP_RETURN_CODE for the env-gated lanes (see the e2e-live entry in
# cmake/TalkServoGates.cmake and the lane test bridge in cmake/TalkServoCrates.cmake):
# "not enabled here" must read as Skipped, never as Passed-quietly or as a failure.
#
# Usage: lane-gate.sh <ENV_NAME> <expected> -- <command> [args...]
#
# Why a script instead of an inline `sh -c "test ... || exit 77; <cmd>"`: the command comes
# from _polyorch_rust_command as a CMake LIST (pixi PATH prefix + host-leak strip). Flattening
# that list into a shell string would re-introduce exactly the button/ctest drift the W4 bridge
# exists to prevent, and would break on any path containing a space.
set -uo pipefail
name="${1:?usage: lane-gate.sh ENV EXPECTED -- cmd ...}"
want="${2:?usage: lane-gate.sh ENV EXPECTED -- cmd ...}"
shift 3   # ENV EXPECTED --
[ "${!name:-}" = "$want" ] || exit 77
exec "$@"
