# TalkServoCrates.cmake — PolyOrch-driven cargo lanes (plan-8 P1 core, P2 matrix; plan-10 vocabulary).
# cargo stays the build engine; the CMake graph DRIVES it (zero compilation rules here).
#
# Lane vocabulary (D27 / docs/plans/2026-10-22-plan10-graph-target-vocabulary.md):
#   every button is  talkservo-<object>-<verb>,  verb in {build, test, clean},
#   object = a lane  <crate>[-<config>]  or a scope  lanes[-<config>],
#   config in {omitted, stub, live} and the suffix appears ONLY where one package has
#   more than one in-graph configuration (today: talkservo-server). The `stub|live` words
#   are the same vocabulary TALKSERVO_PACKAGE_LANE, talkservo.sh --lane, the e2e-stub /
#   e2e-live ctest labels and TS_E2E_FEATURES already use — one word, four surfaces.
#
# One handle per (package, config), tracking that config's DELIVERED artifact:
# BINARY when the package has a bin, LIB otherwise. Splitting lib and bin into two
# handles was wrong rather than merely verbose: the helper accepts exactly one kind per
# handle (PolyOrchRustHelpers.cmake:282 — "pick exactly one of BINARY, STATIC, SHARED,
# EXAMPLE, LIB"), and cargo compiles the lib unit for a `--bin` invocation anyway —
# measured 2026-10-22: one `cargo build -p talkservo-server --bin talkservo-server -v`
# emitted `rustc … --crate-type lib` THEN `rustc … --crate-type bin` in the same call.
# A separate lib handle therefore tracked an artifact nothing in the graph consumed, at
# the price of a name. A second handle for the same package is added only when something
# really consumes the other artifact (the clause reserved for the D8 Beta C-ABI staticlib).
#
# Shape notes inherited from the measured ModuKit crates face (W4 lessons included):
#   * polyorch_rust_test registers a BUILD target, NOT a ctest entry -> the bridge below
#     adds one add_test per lane through the SAME _polyorch_rust_command wrapper
#     (pixi PATH prefix + host-leak strip), so button and ctest can never drift.
#   * one BASE_DIR per cache group. Sharing one dir across the stub and live feature
#     configs of the SAME package made cargo rebuild-thrash (P2 first-red lesson); the
#     groups are named, not tool-named: out/cargo-target/{stub,live}.
#   * consumers never repeat a lane name: TalkServoWeb.cmake reads
#     TALKSERVO_EMBEDDED_WEB_HANDLES, TalkServoInstall.cmake composes
#     talkservo-server-${TALKSERVO_PACKAGE_LANE}, talkservo.sh composes the target name.

option(TALKSERVO_POLYORCH_CRATES "register the PolyOrch-driven cargo lanes in the graph" ON)

set(TALKSERVO_CACHE_ROOT "${CMAKE_SOURCE_DIR}/out/cargo-target")

# talkservo_rust_lane(HANDLE PACKAGE KIND CACHE [FEATURES] [TEST] [TEST_LABELS] [CLEAN])
# Derives manifest path, crate/artifact name, source folder and cache dir from
# PACKAGE + KIND + CACHE, so no field can go stale against another field.
function(talkservo_rust_lane)
  cmake_parse_arguments(PARSE_ARGV 0 L ""
      "HANDLE;PACKAGE;KIND;CACHE;FEATURES;TEST;TEST_LABELS;CLEAN" "")

  foreach(_req HANDLE PACKAGE KIND CACHE)
    if(NOT L_${_req})
      message(FATAL_ERROR "talkservo_rust_lane: ${_req} is required (args: ${ARGN})")
    endif()
  endforeach()
  if(NOT L_KIND MATCHES "^(LIB|BINARY)$")
    message(FATAL_ERROR "talkservo_rust_lane(${L_HANDLE}): KIND must be LIB or BINARY, got '${L_KIND}'")
  endif()
  if(NOT L_CACHE MATCHES "^(stub|live)$")
    message(FATAL_ERROR "talkservo_rust_lane(${L_HANDLE}): CACHE must be stub or live, got '${L_CACHE}'")
  endif()

  set(_folder "crates/${L_PACKAGE}")
  set(_manifest "${CMAKE_SOURCE_DIR}/${_folder}/Cargo.toml")
  set(_base "${TALKSERVO_CACHE_ROOT}/${L_CACHE}")
  string(TOUPPER "${L_CACHE}" _cacheup)

  # cargo's own naming rules: a lib target is the package name with dashes turned into
  # underscores; a bin target keeps the name written in [[bin]] (here: the package name).
  if(L_KIND STREQUAL "BINARY")
    set(_crate "${L_PACKAGE}")
    set(_kindkw BINARY)
  else()
    string(REPLACE "-" "_" _crate "${L_PACKAGE}")
    set(_kindkw LIB)
  endif()

  polyorch_rust_build(
      TARGET   ${L_HANDLE}
      PACKAGE  ${L_PACKAGE}
      CRATE    ${_crate}
      ${_kindkw}
      MANIFEST "${_manifest}"
      BASE_DIR "${_base}"
      FOLDER   "${_folder}")

  # features ride set_features (polyorch_rust_build has no build-time NO_DEFAULT keyword
  # - S1 forensics corrected a wrong guess). Empty FEATURES = package defaults.
  if(L_FEATURES)
    polyorch_rust_set_features(TARGET ${L_HANDLE} NO_DEFAULT_FEATURES FEATURES "${L_FEATURES}")
  endif()

  # P2 link-driver lesson (conda toolchain): the helper's host-leak strip removes
  # CC/CXX, and CXX_<triple> only steers cc-rs compilations - cargo's final LINK would
  # fall back to the conda `cc` wrapper, whose static-libstdc++ combination dies on
  # __libc_single_threaded (undefined symbol) for C++-linking lanes. The daily flow works
  # because activation sets CC=conda-clang; pin the equivalent, and pin it to plain `cc`,
  # NOT to CMAKE_CXX_COMPILER (clang++-23 resolves the conda-prefixed ld with an old
  # built-in sysroot, same missing symbol).
  string(TOUPPER "${POLYORCH_RUST_HOST_TARGET}" _linktr)
  string(REPLACE "-" "_" _linktr "${_linktr}")
  find_program(TALKSERVO_CONDA_CC cc REQUIRED)
  # NO_PROXY for pypi (both cases): mediasoup-sys build.rs pip-install hits a MITM proxy
  # that breaks pypi TLS specifically on this class of host (gitee/github still route fine).
  polyorch_rust_set_env_vars(TARGET ${L_HANDLE}
      "CARGO_TARGET_${_linktr}_LINKER=${TALKSERVO_CONDA_CC}"
      "NO_PROXY=*"
      "no_proxy=*")

  set_property(GLOBAL APPEND PROPERTY TALKSERVO_LANE_HANDLES "${L_HANDLE}")
  set_property(GLOBAL APPEND PROPERTY TALKSERVO_LANE_HANDLES_${_cacheup} "${L_HANDLE}")
  if(L_FEATURES MATCHES "embedded-web")
    set_property(GLOBAL APPEND PROPERTY TALKSERVO_EMBEDDED_WEB_HANDLES "${L_HANDLE}")
  endif()

  if(L_CLEAN)
    # Package-scoped by construction (cargo clean -p inside THIS cache group), so
    # talkservo-core-clean cannot touch server artifacts that share the group. The
    # cache-root nuke is talkservo-lanes-clean: two actions, two honest names.
    _polyorch_rust_forward_env(TRIPLE "${POLYORCH_RUST_HOST_TARGET}" OUT_ENV _fenv)
    _polyorch_rust_command(_clean SUBCOMMAND clean -p "${L_PACKAGE}" --target-dir "${_base}" ENV "${_fenv}")
    add_custom_target(${L_HANDLE}-clean
        COMMAND ${_clean}
        COMMENT "cargo clean -p ${L_PACKAGE} (${L_CACHE} cache group)"
        FOLDER "${_folder}")
  endif()

  if(NOT L_TEST STREQUAL "ON")
    message(STATUS "talkservo lane ${L_HANDLE}: ${L_PACKAGE} [${L_KIND} @ ${L_CACHE}] registered (no test verb)")
    return()
  endif()

  polyorch_rust_test(
      PACKAGE  ${L_PACKAGE}
      NAME     ${L_HANDLE}-test
      TARGET   ${L_HANDLE}
      MANIFEST "${_manifest}"
      BASE_DIR "${_base}"
      FOLDER   "${_folder}")

  # W4 ctest bridge: same argv as the -test button (plus the lane's feature flags),
  # same env wrapper.
  _polyorch_rust_forward_env(TRIPLE "${POLYORCH_RUST_HOST_TARGET}" OUT_ENV _fenv)
  set(_argv test --package "${L_PACKAGE}" --manifest-path "${_manifest}" --target-dir "${_base}")
  if(L_FEATURES)
    list(APPEND _argv --no-default-features --features "${L_FEATURES}")
  endif()
  _polyorch_rust_command(_cmd SUBCOMMAND ${_argv} ENV "${_fenv}")
  add_test(NAME ${L_HANDLE}-test COMMAND ${_cmd})
  set_tests_properties(${L_HANDLE}-test PROPERTIES
      WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
      SKIP_RETURN_CODE 77
      LABELS "${L_TEST_LABELS}")
  message(STATUS "talkservo lane ${L_HANDLE}: ${L_PACKAGE} [${L_KIND} @ ${L_CACHE}] + test (${L_TEST_LABELS})")
endfunction()

# talkservo_crates() - call AFTER talkservo_ensure_polyorch_rust().
function(talkservo_crates)
  if(NOT TALKSERVO_POLYORCH_CRATES)
    return()
  endif()
  if(NOT COMMAND polyorch_rust_test OR NOT POLYORCH_RUST_FOUND)
    message(STATUS "talkservo crates: rust face not available (sentinel/skip path) -- off")
    return()
  endif()

  talkservo_rust_lane(HANDLE talkservo-core        PACKAGE talkservo-core   KIND LIB
      CACHE stub TEST ON TEST_LABELS "rust;headless" CLEAN ON)
  talkservo_rust_lane(HANDLE talkservo-sfu         PACKAGE talkservo-sfu    KIND LIB
      CACHE stub FEATURES stub-media TEST ON TEST_LABELS "rust;headless" CLEAN ON)
  # The delivered shape of the everyday lane is the binary (rust-embed bakes web/dist at
  # compile time); the ordering edge onto the Node lane is added by TalkServoWeb.cmake
  # from TALKSERVO_EMBEDDED_WEB_HANDLES, never from a hand-written name list.
  talkservo_rust_lane(HANDLE talkservo-server-stub PACKAGE talkservo-server KIND BINARY
      CACHE stub FEATURES stub-media,embedded-web TEST ON TEST_LABELS "rust;headless" CLEAN ON)
  # The real SFU backend: build-only for now. Its test verb lands in plan-10 T4; until
  # then `pixi run test-sfu` still owns it, which modules/09 records either way.
  talkservo_rust_lane(HANDLE talkservo-server-live PACKAGE talkservo-server KIND BINARY
      CACHE live FEATURES sfu-mediasoup,embedded-web TEST OFF CLEAN ON)

  # Daily aggregate: every stub-group lane, never the live one (the C++ worker stays
  # opt-in per lane). ALL is deliberate (plan-10 F1=B): with no default goal a bare
  # `cmake --build` built NOTHING and still exited 0 — green from a mechanism that never
  # ran (PIT-27 family), which is also why pixi.toml had to enumerate six target names.
  get_property(_stub GLOBAL PROPERTY TALKSERVO_LANE_HANDLES_STUB)
  set(_stub_deps "")
  foreach(_h ${_stub})
    list(APPEND _stub_deps "${_h}-build")
  endforeach()
  add_custom_target(talkservo-lanes-stub-build ALL DEPENDS ${_stub_deps}
      COMMENT "lanes-stub aggregate: ${_stub_deps}")

  # one button for the whole cache root (both groups live under it now); the surgical
  # path is the per-lane cargo clean above.
  polyorch_rust_clean(NAME talkservo-lanes-clean BASE_DIR "${TALKSERVO_CACHE_ROOT}" FOLDER "crates")
endfunction()
