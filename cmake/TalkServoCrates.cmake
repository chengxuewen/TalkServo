# TalkServoCrates.cmake — PolyOrch-driven cargo lanes (plan-8 P1: core lane only).
# cargo stays the build engine; the CMake graph DRIVES it (zero compilation rules here).
# Shape copied from the measured ModuKit crates face (W4 lessons included):
#   * polyorch_rust_test registers a BUILD target, NOT a ctest entry -> the bridge
#     below adds one add_test per lane through the SAME _polyorch_rust_command wrapper
#     (pixi PATH prefix + host-leak strip), so button and ctest can never drift.
#   * shared BASE_DIR (out/cargo-target, gitignored by the `out` rule).

option(TALKSERVO_POLYORCH_CRATES "register the PolyOrch-driven cargo lanes in the graph" ON)

# Lane table - <package>@<position-mirror folder>@<kind>@<cargo lib name>.
# P1 lights core only; sfu/server lanes land at P2 (dual feature configs).
set(TALKSERVO_RUST_LANES
    "talkservo-core@crates/talkservo-core@lib@talkservo_core")

# talkservo_crates() - call AFTER talkservo_ensure_polyorch_rust().
function(talkservo_crates)
  if(NOT TALKSERVO_POLYORCH_CRATES)
    return()
  endif()
  if(NOT COMMAND polyorch_rust_test OR NOT POLYORCH_RUST_FOUND)
    message(STATUS "talkservo crates: rust face not available (sentinel/skip path) -- off")
    return()
  endif()

  set(_base "${CMAKE_SOURCE_DIR}/out/cargo-target")

  foreach(_entry ${TALKSERVO_RUST_LANES})
    string(REPLACE "@" ";" _kv "${_entry}")
    list(GET _kv 0 _pkg)
    list(GET _kv 1 _fold)
    list(GET _kv 2 _kind)
    list(GET _kv 3 _crate)
    string(TOUPPER "${_kind}" _kindw)
    set(_manifest "${CMAKE_SOURCE_DIR}/${_fold}/Cargo.toml")

    polyorch_rust_build(
        TARGET   ${_pkg}
        PACKAGE  ${_pkg}
        CRATE    ${_crate}
        ${_kindw}
        MANIFEST "${_manifest}"
        BASE_DIR "${_base}"
        FOLDER   "${_fold}")

    polyorch_rust_test(
        PACKAGE  ${_pkg}
        NAME     ${_pkg}-test
        TARGET   ${_pkg}
        MANIFEST "${_manifest}"
        BASE_DIR "${_base}"
        FOLDER   "${_fold}")

    polyorch_rust_clean(
        NAME     ${_pkg}-clean
        BASE_DIR "${_base}"
        FOLDER   "${_fold}")

    # W4 ctest bridge: same argv as the -test button, same env wrapper.
    _polyorch_rust_forward_env(TRIPLE "${POLYORCH_RUST_HOST_TARGET}" OUT_ENV _fenv)
    _polyorch_rust_command(_cmd SUBCOMMAND
        test --package "${_pkg}" --manifest-path "${_manifest}" --target-dir "${_base}"
        ENV "${_fenv}")
    add_test(NAME ${_pkg}-test COMMAND ${_cmd})
    set_tests_properties(${_pkg}-test PROPERTIES
        WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
        SKIP_RETURN_CODE 77
        LABELS "rust;headless")
  endforeach()
endfunction()
