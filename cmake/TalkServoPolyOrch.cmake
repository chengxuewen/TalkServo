# TalkServoPolyOrch.cmake — thin wrapper over the PolyOrch helper surface (D25/plan-8 P1).
# Keep this file small: it embeds PolyOrch and verifies the helpers exist. Per-lane
# wiring (web / gates / install) grows here as talkservo_* functions per phase
# (ModuKit D66 / VisiaEngine D39 wrapper shape).

# talkservo_ensure_polyorch(): sentinel embed, first-wins (D39/D66 semantics).
#   - A superbuild that already added PolyOrch wins; we reuse its copy.
#   - Otherwise self-embed the submodule at its gitlink pin (the gitlink is the sole
#     version authority — this wrapper NEVER fetches, never tracks a branch).
#   - TALKSERVO_POLYORCH_DIR: escape hatch for offline/superbuild checkouts.
function(talkservo_ensure_polyorch)
  if(COMMAND polyorch_rust_setup)
    message(STATUS "talkservo: reusing PolyOrch provided by the outer scope (first-wins)")
    return()
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

  add_subdirectory("${_po}" "${CMAKE_BINARY_DIR}/_polyorch" SYSTEM)

  if(NOT COMMAND polyorch_rust_setup)
    message(FATAL_ERROR
      "PolyOrch embedded but its helpers are absent - check the gitlink pin "
      "(5945f2a verified against the audited 0.3.0 snapshot 2026-10-19)")
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
    polyorch_pixi_setup(REPORT)
    polyorch_rust_setup(FROM pixi REQUIRED)
  else()
    polyorch_rust_setup(FROM system REQUIRED)
  endif()
  message(STATUS "talkservo: rust face ready (pixi=${TALKSERVO_USE_PIXI}, cargo=${POLYORCH_RUST_CARGO})")
endfunction()
