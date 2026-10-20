# TalkServoCrates.cmake — PolyOrch-driven cargo lanes (plan-8 P1 core, P2 full matrix).
# cargo stays the build engine; the CMake graph DRIVES it (zero compilation rules here).
# P2 additions: sfu + server lanes on the STUB config (daily/test gate), server BINARY
# handle, and a mediasoup-feature ARTIFACT handle (build-only — its test verb stays the
# `test-sfu` cargo task until P3 folds labels; dual-gate D16/D5 discipline preserved).
# Shape copied from the measured ModuKit crates face (W4 lessons included):
#   * polyorch_rust_test registers a BUILD target, NOT a ctest entry -> the bridge
#     below adds one add_test per lane through the SAME _polyorch_rust_command wrapper
#     (pixi PATH prefix + host-leak strip), so button and ctest can never drift.
#   * shared BASE_DIR (out/cargo-target, gitignored by the `out` rule).

option(TALKSERVO_POLYORCH_CRATES "register the PolyOrch-driven cargo lanes in the graph" ON)

# Lane table - <handle>@<cargo package>@<folder-mirror>@<kind>@<crate lib name>@<base>@<features>|<test 0/1>.
# handle != package for alias lanes (server-bin / mediasoup artifact share the package
# with the stub lane; cargo --package must carry the REAL name - P2 first-red lesson).
# Empty features = package defaults. features set => NO_DEFAULT_FEATURES + FEATURES
# (dual-gate: exactly one backend per compile unit, Cargo.toml cfg-assert enforces).
# One BASE_DIR per FEATURE CONFIG (P2 first-red lesson): artifact output names must be
# unique per cargo-target dir, and sharing one dir across feature sets makes cargo
# rebuild-thrash; stub/default ride the main cache, mediasoup gets its own.
set(TALKSERVO_RUST_LANES
    "talkservo-core@talkservo-core@crates/talkservo-core@lib@talkservo_core@out/cargo-target@|1"
    "talkservo-sfu@talkservo-sfu@crates/talkservo-sfu@lib@talkservo_sfu@out/cargo-target@stub-media|1"
    "talkservo-server@talkservo-server@crates/talkservo-server@lib@talkservo_server@out/cargo-target@stub-media,embedded-web|1"
    "talkservo-server-bin@talkservo-server@crates/talkservo-server@binary@talkservo-server@out/cargo-target@stub-media,embedded-web|0"
    "talkservo-server-mediasoup@talkservo-server@crates/talkservo-server@binary@talkservo-server@out/cargo-target-mediasoup@sfu-mediasoup,embedded-web|0")

# talkservo_crates() - call AFTER talkservo_ensure_polyorch_rust().
function(talkservo_crates)
  if(NOT TALKSERVO_POLYORCH_CRATES)
    return()
  endif()
  if(NOT COMMAND polyorch_rust_test OR NOT POLYORCH_RUST_FOUND)
    message(STATUS "talkservo crates: rust face not available (sentinel/skip path) -- off")
    return()
  endif()

  foreach(_entry ${TALKSERVO_RUST_LANES})
    string(REGEX REPLACE "\\|.*$" "" _specs "${_entry}")
    string(REGEX REPLACE "^.*\\|" "" _dotest "${_entry}")
    string(REPLACE "@" ";" _kv "${_specs}")
    list(GET _kv 0 _pkg)      # handle (button/target/ctest naming)
    list(GET _kv 1 _cpkg)    # real cargo package (--package argv)
    list(GET _kv 2 _fold)
    list(GET _kv 3 _kind)
    list(GET _kv 4 _crate)
    list(GET _kv 5 _base)
    list(GET _kv 6 _feat)
    set(_base "${CMAKE_SOURCE_DIR}/${_base}")
    string(TOUPPER "${_kind}" _kindw)
    set(_manifest "${CMAKE_SOURCE_DIR}/${_fold}/Cargo.toml")

    # features ride set_features below (per-handle properties; polyorch_rust_build
    # has no build-time NO_DEFAULT keyword - S1 forensics corrected a wrong guess).
    polyorch_rust_build(
        TARGET   ${_pkg}
        PACKAGE  ${_cpkg}
        CRATE    ${_crate}
        ${_kindw}
        MANIFEST "${_manifest}"
        BASE_DIR "${_base}"
        FOLDER   "${_fold}")

    if(_feat)
      polyorch_rust_set_features(TARGET ${_pkg} NO_DEFAULT_FEATURES FEATURES "${_feat}")
    endif()

    # P2 link-driver lesson (conda toolchain): the helper's host-leak strip removes
    # CC/CXX, and CXX_<triple> only steers cc-rs compilations - cargo's final LINK
    # would fall back to the conda `cc` wrapper, whose static-libstdc++ combination
    # dies on __libc_single_threaded (undefined symbol) for C++-linking lanes. The
    # daily flow works because activation sets CC=conda-clang. Pin the equivalent:
    # linker = the configure-time C++ compiler (conda clang under pixi env dev).
    string(TOUPPER "${POLYORCH_RUST_HOST_TARGET}" _linktr)
    string(REPLACE "-" "_" _linktr "${_linktr}")
    # NOTE: must be `cc`, NOT the CMAKE_CXX_COMPILER (clang++-23): the ++23 driver
    # resolves the conda-prefixed x86_64-conda-linux-gnu-ld (old built-in sysroot,
    # no __libc_single_threaded); plain cc links like the daily activation flow.
    find_program(TALKSERVO_CONDA_CC cc REQUIRED)
    # NO_PROXY for pypi (both cases): mediasoup-sys build.rs pip-install hits a
    # MITM proxy that breaks pypi TLS specifically on this class of host
    # (gitee/github still route fine through it) - surgical bypass, deterministic build.
    polyorch_rust_set_env_vars(TARGET ${_pkg}
        "CARGO_TARGET_${_linktr}_LINKER=${TALKSERVO_CONDA_CC}"
        "NO_PROXY=*"
        "no_proxy=*")

    if(NOT _dotest STREQUAL "1")
      continue()  # artifact handles: build-only; test verb lives elsewhere
    endif()

    polyorch_rust_test(
        PACKAGE  ${_cpkg}
        NAME     ${_pkg}-test
        TARGET   ${_pkg}
        MANIFEST "${_manifest}"
        BASE_DIR "${_base}"
        FOLDER   "${_fold}")

    # W4 ctest bridge: same argv as the -test button (plus the lane's feature flags),
    # same env wrapper.
    _polyorch_rust_forward_env(TRIPLE "${POLYORCH_RUST_HOST_TARGET}" OUT_ENV _fenv)
    set(_argv test --package "${_cpkg}" --manifest-path "${_manifest}" --target-dir "${_base}")
    if(_feat)
      list(APPEND _argv --no-default-features --features "${_feat}")
    endif()
    _polyorch_rust_command(_cmd SUBCOMMAND ${_argv} ENV "${_fenv}")
    add_test(NAME ${_pkg}-test COMMAND ${_cmd})
    set_tests_properties(${_pkg}-test PROPERTIES
        WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
        SKIP_RETURN_CODE 77
        LABELS "rust;headless")
  endforeach()

  # clean buttons: one per feature-config cache (honest rm -rf, idempotent).
  polyorch_rust_clean(NAME lanes-clean BASE_DIR "${CMAKE_SOURCE_DIR}/out/cargo-target" FOLDER "crates")
  polyorch_rust_clean(NAME lanes-mediasoup-clean BASE_DIR "${CMAKE_SOURCE_DIR}/out/cargo-target-mediasoup" FOLDER "crates")
endfunction()
