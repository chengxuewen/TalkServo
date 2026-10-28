# TalkServoPolyOrch.cmake — thin wrapper over the PolyOrch helper surface (D25/plan-8 P1).
# Keep this file small: it embeds PolyOrch and verifies the helpers exist. Per-lane
# wiring (web / gates / install) grows here as talkservo_* functions per phase
# (ModuKit D66 / VisiaEngine D39 wrapper shape).

# talkservo_ensure_polyorch(): PolyOrch::PolyOrch sentinel embed, first-wins (D39/D66/plan-11).
#   - A superbuild that already added PolyOrch wins only if it carries the versioned sentinel.
#   - Otherwise self-embed the submodule at its gitlink pin (the gitlink is the sole
#     version authority — this wrapper NEVER fetches, never tracks a branch).
#   - TALKSERVO_POLYORCH_DIR: escape hatch for offline/superbuild checkouts.
function(talkservo_require_polyorch_sentinel)
  if(NOT TARGET PolyOrch::PolyOrch)
    message(FATAL_ERROR
      "PolyOrch was added but did not install PolyOrch::PolyOrch. TalkServo requires PolyOrch 0.7.1.")
  endif()

  get_property(_po_dir TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_CMAKE_DIR)
  get_property(_po_ver TARGET PolyOrch::PolyOrch PROPERTY INTERFACE_POLYORCH_VERSION)

  if(NOT _po_dir)
    message(FATAL_ERROR
      "PolyOrch was added but did not install PolyOrch::PolyOrch with INTERFACE_POLYORCH_CMAKE_DIR.")
  endif()

  if(NOT _po_ver OR _po_ver VERSION_LESS 0.7.1)
    message(FATAL_ERROR
      "TalkServo requires PolyOrch >= 0.7.1 (found '${_po_ver}'). Run git submodule update --init --recursive.")
  endif()
endfunction()

function(talkservo_ensure_polyorch)
  if(COMMAND polyorch_rust_setup)
    talkservo_require_polyorch_sentinel()
    message(STATUS "talkservo: reusing PolyOrch provided by the outer scope (first-wins)")
    return()
  endif()

  if(TARGET PolyOrch::PolyOrch)
    talkservo_require_polyorch_sentinel()
  endif()

  if(TALKSERVO_POLYORCH_DIR)
    set(_po "${TALKSERVO_POLYORCH_DIR}")
  else()
    set(_po "${CMAKE_CURRENT_FUNCTION_LIST_DIR}/../PolyOrch")
  endif()
  if(NOT EXISTS "${_po}/CMakeLists.txt")
    message(FATAL_ERROR
      "PolyOrch submodule missing at ${_po} - run: git submodule update --init "
      "(gitlink pin; or set -DTALKSERVO_POLYORCH_DIR=<checkout>)")
  endif()

  add_subdirectory("${_po}" "${CMAKE_BINARY_DIR}/_polyorch" SYSTEM EXCLUDE_FROM_ALL)

  talkservo_require_polyorch_sentinel()
  if(NOT COMMAND polyorch_rust_setup)
    message(FATAL_ERROR
      "TalkServo requires PolyOrch >= 0.7.1 but polyorch_rust_setup is absent after embedding.")
  endif()
  message(STATUS "talkservo: PolyOrch self-embedded at its gitlink pin")
endfunction()

option(TALKSERVO_USE_PIXI "build toolchain comes from the pinned pixi env (D5/D25)" ON)

# talkservo_ensure_polyorch_rust(): translate the USE_PIXI switch onto PolyOrch's rust
# setup. Call AFTER talkservo_ensure_polyorch(). Divergence from sisters (recorded in
# docs/modules/09 P1.0 posture): NO COPY_SCRIPTS — our pixi.sh/pixi.bat stay repo-owned
# (subshell-entry semantics); PolyOrch's template would swap in source-activate files.
function(talkservo_ensure_polyorch_rust)
  if(NOT COMMAND polyorch_rust_setup)
    message(FATAL_ERROR "call talkservo_ensure_polyorch() first - PolyOrch helpers absent")
  endif()
  if(TALKSERVO_USE_PIXI)
    if(NOT COMMAND polyorch_pixi_setup)
      message(FATAL_ERROR "TALKSERVO_USE_PIXI needs the PolyOrch pixi helper face")
    endif()
    # ENVIRONMENT dev: the conda toolchain tuning (CC/CXX clang++-23,
    # LD_LIBRARY_PATH, libclang) lives in the dev env (modules/09 §3); the
    # default env only carries cmake (P1.0) - linking mediasoup needs dev.
    polyorch_pixi_setup(ENVIRONMENT dev REPORT)
    polyorch_rust_setup(FROM pixi REQUIRED)
  else()
    polyorch_rust_setup(FROM system REQUIRED)
  endif()
  message(STATUS "talkservo: rust face ready (pixi=${TALKSERVO_USE_PIXI}, cargo=${POLYORCH_RUST_CARGO})")
endfunction()

# talkservo_mediasoup_toolchain_env(OUT <var> [WARN_TARGET <name>]) - the env trio that
# mediasoup-sys' build.rs→meson needs to build the C++ worker from a NOT-activated shell
# (native-build-env-hardening, 2026-10-27): CC/CXX = the conda clang that pixi.toml
# [activation.env] pins (the supplier wrapper strips inherited CC/CXX by design, and the
# env bin carries no bare c++ symlink, so meson would fall back to the system gcc and die
# on C++20 concepts); LD_LIBRARY_PATH = the conda lib dir for flatc's runtime libstdc++
# (host GLIBCXX is older than what conda-compiled flatc loads). Values derive from the
# resolved toolchain's own bin dir (POLYORCH_RUST_BIN_DIR) - no .pixi/envs literals.
# Empty result + WARNING when the files are absent (caller then gets the historical
# activated-shell behavior; the divergence stays visible).
function(talkservo_mediasoup_toolchain_env)
  cmake_parse_arguments(PARSE_ARGV 0 E "" "OUT;WARN_TARGET" "")
  set(_env "")
  set(_cc "${POLYORCH_RUST_BIN_DIR}/clang")
  set(_ld "${POLYORCH_RUST_BIN_DIR}/../lib")
  if(EXISTS "${_cc}" AND EXISTS "${_ld}/libstdc++.so.6")
    set(_env "CC=${_cc}" "CXX=${POLYORCH_RUST_BIN_DIR}/clang++-23" "LD_LIBRARY_PATH=${_ld}")
  else()
    message(WARNING "talkservo ${E_WARN_TARGET}: conda clang toolchain not found at ${_cc} -"
      " the mediasoup C++ worker build needs an activated pixi shell (CC/CXX)")
  endif()
  set(${E_OUT} "${_env}" PARENT_SCOPE)
endfunction()