#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/bare-polyorch" "$TMP/old-polyorch" "$TMP/test-project"

cat > "$TMP/bare-polyorch/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.25)
project(PolyOrch VERSION 0.3.0 LANGUAGES NONE)
EOF

cat > "$TMP/old-polyorch/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.25)
project(PolyOrch VERSION 0.3.0 LANGUAGES NONE)
add_library(PolyOrch::PolyOrch INTERFACE IMPORTED GLOBAL)
set_property(TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_CMAKE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
set_property(TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_VERSION "${PROJECT_VERSION}")
EOF

cat > "$TMP/test-project/CMakeLists.txt" <<EOF
cmake_minimum_required(VERSION 3.25)
project(TalkServoPolyOrchGateTest LANGUAGES CXX)
include("$ROOT/cmake/TalkServoPolyOrch.cmake")
talkservo_ensure_polyorch()
EOF

for label in bare old; do
  dir="$TMP/${label}-polyorch"
  build="$TMP/build-${label}"
  log="$TMP/log-${label}"
  if cmake -S "$TMP/test-project" -B "$build" -DTALKSERVO_POLYORCH_DIR="$dir" > "$log" 2>&1; then
    echo "FAIL: PolyOrch ${label} fixture was accepted"
    exit 1
  fi
done

grep -qi 'PolyOrch was added but did not install PolyOrch::PolyOrch' "$TMP/log-bare"
grep -qi 'TalkServo requires PolyOrch >= 0.7.1' "$TMP/log-old"

echo PASS
