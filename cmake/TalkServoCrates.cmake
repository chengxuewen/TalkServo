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
#   * PolyOrch 0.7.1 can register the button and ctest entry from one CTEST call.
#     Ungated Rust lanes use that path. Runtime-gated live lanes keep a wrapper so
#     TS_LANE_LIVE can still decide at run time.
#   * one BASE_DIR per cache group. Sharing one dir across the stub and live feature
#     configs of the SAME package made cargo rebuild-thrash (P2 first-red lesson); the
#     groups are named, not tool-named: out/cargo-target/{stub,live}.
#   * consumers never repeat a lane name: TalkServoWeb.cmake reads
#     TALKSERVO_EMBEDDED_WEB_HANDLES, TalkServoInstall.cmake composes
#     talkservo-server-${TALKSERVO_PACKAGE_LANE}, talkservo.sh composes the target name.

option(TALKSERVO_POLYORCH_CRATES "register the PolyOrch-driven cargo lanes in the graph" ON)
option(TALKSERVO_LIVE_LANE "register the talkservo-server-live lane (C++ worker build is opt-in)" OFF)
# When the packaging face selects the live lane, the live lane must exist - default the
# graph knob to follow it so the deploy/package flow needs one switch, not two
# (TALKSERVO_PACKAGE_LANE lives in TalkServoInstall.cmake and defaults to live).
if(DEFINED TALKSERVO_PACKAGE_LANE AND TALKSERVO_PACKAGE_LANE STREQUAL "live")
  set(TALKSERVO_LIVE_LANE ON CACHE BOOL "forced by TALKSERVO_PACKAGE_LANE=live" FORCE)
endif()

set(TALKSERVO_CACHE_ROOT "${CMAKE_SOURCE_DIR}/out/cargo-target")

# talkservo_rust_lane(HANDLE PACKAGE KIND CACHE [FEATURES] [TEST] [TEST_LABELS]
#                       [TEST_GATE <ENV>] [TEST_GATE_VALUE <v>] [CLEAN])
# TEST_GATE defers the ctest entry to run time: unless ENV=v, the entry exits 77 and ctest
# reports Skipped. That is how the heavy live lane stays registered in the graph without
# ever entering the daily `ctest -L "rust|web"` set (same mechanism e2e-live uses).
# Derives manifest path, crate/artifact name, source folder and cache dir from
# PACKAGE + KIND + CACHE, so no field can go stale against another field.
function(talkservo_rust_lane)
  cmake_parse_arguments(PARSE_ARGV 0 L ""
      "HANDLE;PACKAGE;KIND;CACHE;FEATURES;TEST;TEST_LABELS;TEST_GATE;TEST_GATE_VALUE;CLEAN" "")

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
  #
  # Plan-11 review correction: supplier CTEST has no lane ENV keyword, so the pin cannot
  # be carried into an ungated `polyorch_rust_test(... CTEST ...)` call. To keep build and
  # test aligned, only C++-linking / runtime-gated lanes carry this pin. The stub lanes
  # are unpinned on both sides; the live mediasoup lane remains pinned in its hand-wrapped
  # ctest entry.
  set(_laneenv "")
  if(L_TEST_GATE OR L_FEATURES MATCHES "sfu-mediasoup")
    string(TOUPPER "${POLYORCH_RUST_HOST_TARGET}" _linktr)
    string(REPLACE "-" "_" _linktr "${_linktr}")
    # Pin the link driver: under the pixi env, a bare `cc` on PATH resolves to
    # .pixi/envs/dev/bin/cc -> x86_64-conda-linux-gnu-cc, whose built-in sysroot lacks
    # __libc_single_threaded. The absolute path is what works.
    find_program(TALKSERVO_CONDA_CC cc REQUIRED)
    # C++ worker toolchain (2026-10-27 native-build-env-hardening): the worker also builds
    # via this lane's cargo, so it needs the same pins the gate carries - see
    # talkservo_mediasoup_toolchain_env for what is pinned and why (strip + no bare c++
    # symlink in the env bin = meson falls back to system gcc without this).
    talkservo_mediasoup_toolchain_env(OUT _ms_env WARN_TARGET "lane ${L_HANDLE}")
    list(APPEND _laneenv ${_ms_env})
    # NO_PROXY for pypi (both cases): mediasoup-sys build.rs pip-install hits a MITM proxy
    # that breaks pypi TLS specifically on this class of host (gitee/github still route fine).
    list(APPEND _laneenv
        "CARGO_TARGET_${_linktr}_LINKER=${TALKSERVO_CONDA_CC}"
        "NO_PROXY=*"
        "no_proxy=*")
    polyorch_rust_set_env_vars(TARGET ${L_HANDLE} ${_laneenv})
  endif()

  set_property(GLOBAL APPEND PROPERTY TALKSERVO_LANE_HANDLES "${L_HANDLE}")
  set_property(GLOBAL APPEND PROPERTY TALKSERVO_LANE_HANDLES_${_cacheup} "${L_HANDLE}")
  if(L_FEATURES MATCHES "embedded-web")
    set_property(GLOBAL APPEND PROPERTY TALKSERVO_EMBEDDED_WEB_HANDLES "${L_HANDLE}")
  endif()

  # No per-lane `cargo clean -p`: it is refused on a SHARED cache group directory unless that
  # directory carries a valid CACHEDIR.TAG (measured: out/cargo-target/stub has debug/, tmp/ and
  # .rustc_info.json but no tag, so talkservo-sfu-clean failed with "cannot clean ... missing or
  # invalid CACHEDIR.TAG" - cleaning is aborted by cargo on purpose, to avoid deleting files it
  # does not own). The surgical path is a documented shell command instead; the graph only offers
  # deletion it can guarantee: rm -rf per cache group (talkservo-lanes-{stub,live}-clean) and of
  # the cache root (talkservo-lanes-clean), all via polyorch_rust_clean.
  # (plan-10 F2 revised: the original O2 choice is retracted on this evidence - see PIT-40.)
  if(L_CLEAN)
    # accepted for interface stability, deliberately inert: see the note above
  endif()

  if(NOT L_TEST STREQUAL "ON")
    message(STATUS "talkservo lane ${L_HANDLE}: ${L_PACKAGE} [${L_KIND} @ ${L_CACHE}] registered (no test verb)")
    return()
  endif()

  set(_test_args "")
  if(L_FEATURES)
    set(_test_args --no-default-features --features "${L_FEATURES}")
  endif()

  if(L_TEST_GATE)
    polyorch_rust_test(
        PACKAGE  ${L_PACKAGE}
        NAME     ${L_HANDLE}-test
        TARGET   ${L_HANDLE}
        MANIFEST "${_manifest}"
        BASE_DIR "${_base}"
        FOLDER   "${_folder}"
        ARGS     ${_test_args})

    # Runtime-gated live lanes keep a hand-written ctest wrapper so the
    # TS_LANE_LIVE condition can still decide at run time. The wrapper uses
    # the same argv, feature flags, and linker pin as the lane's build path.
    _polyorch_rust_forward_env(TRIPLE "${POLYORCH_RUST_HOST_TARGET}" OUT_ENV _fenv)
    set(_argv test --package "${L_PACKAGE}" --manifest-path "${_manifest}" --target-dir "${_base}")
    if(L_FEATURES)
      list(APPEND _argv --no-default-features --features "${L_FEATURES}")
    endif()

    # ENV must carry _laneenv too: the build rule gets the linker pin through the target
    # property, but this command is composed independently — and `cargo test` RE-LINKS the
    # bin and harnesses. Without the pin the relink resolves `cc` off PATH (conda-cc) and the
    # mediasoup lane dies on __libc_single_threaded. Measured 2026-10-22: the live test entry
    # failed 3 link units while the same argv with an absolute gcc linked in 4m40s — and the
    # old `pixi run test-sfu` could never have passed, which is why nobody had run it.
    _polyorch_rust_command(_cmd SUBCOMMAND ${_argv} ENV ${_fenv} ${_laneenv})
    add_test(NAME ${L_HANDLE}-test
        COMMAND bash "${CMAKE_SOURCE_DIR}/scripts/lane-gate.sh"
                "${L_TEST_GATE}" "${L_TEST_GATE_VALUE}" -- ${_cmd})
    set_tests_properties(${L_HANDLE}-test PROPERTIES
        WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
        SKIP_RETURN_CODE 77
        LABELS "${L_TEST_LABELS}")
  else()
    # Ungated Rust lanes use supplier CTEST so the IDE button and ctest entry
    # come from the same command vector.
    polyorch_rust_test(
        PACKAGE  ${L_PACKAGE}
        NAME     ${L_HANDLE}-test
        TARGET   ${L_HANDLE}
        MANIFEST "${_manifest}"
        BASE_DIR "${_base}"
        FOLDER   "${_folder}"
        ARGS     ${_test_args}
        CTEST
        CTEST_NAME ${L_HANDLE}-test
        CTEST_LABELS "${L_TEST_LABELS}"
        CTEST_WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
        CTEST_SKIP_RETURN_CODE 77)
  endif()

  if(L_FEATURES MATCHES "embedded-web")
    # Register the need; TalkServoWeb.cmake supplies the fixture. A fresh tree must be able
    # to run `ctest -L "rust|web"` with no prior build step: rust-embed fails at COMPILE time
    # ("folder web/dist does not exist") when nobody built the SPA first, and the -build
    # ordering edge alone does not cover this command path. The plan-10 fresh-clone drill
    # caught it on 2026-10-22: the primary tree was green only because web/dist already
    # existed there — a warm tree hiding a broken edge is exactly the PIT-27 family.
    set_property(GLOBAL APPEND PROPERTY TALKSERVO_WEBDIST_TESTS "${L_HANDLE}-test")
  endif()
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
  # The real SFU backend, and the only target that compiles the C++ worker. Its test verb
  # lives in the graph now (plan-10 T4) but is run-time gated: TS_LANE_LIVE=1 or the entry
  # reports Skipped, because a worker build is minutes and the daily lane must stay seconds.
  # The label is `mediasoup`, NOT `rust-live`: ctest's -L is a partial regex, so a label
  # like rust-live is matched by the daily `-L "rust|web"` filter too (measured: 5 entries
  # instead of 4). A label with no substring overlap keeps `ctest -L rust` meaning exactly
  # what it meant before this lane existed.
  # The live lane is graph-opt-in (TALKSERVO_LIVE_LANE, default OFF): the C++ worker build
  # is minutes and needs the mediasoup toolchain face, so presets keep it out of the daily
  # surface; the live preset and the deploy/package flow turn it on. The lane's ctest entry
  # remains runtime-gated by TS_LANE_LIVE even when registered (default skips in seconds).
  if(TALKSERVO_LIVE_LANE)
    talkservo_rust_lane(HANDLE talkservo-server-live PACKAGE talkservo-server KIND BINARY
        CACHE live FEATURES sfu-mediasoup,embedded-web
        TEST ON TEST_LABELS "mediasoup;headless;slow"
        TEST_GATE TS_LANE_LIVE TEST_GATE_VALUE 1 CLEAN ON)
  endif()

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
  # add_custom_target has NO FOLDER keyword; a stray one is silently absorbed (measured: the
  # rule's DESC became "crates/talkservo-server" and no IDE folder was set at all). FOLDER is a
  # target PROPERTY, so it is set that way - everywhere in this repo, including for the helper-
  # created Node targets, which only take it from PolyOrch_RUST_FOLDER_ROOT.
  set_property(TARGET talkservo-lanes-stub-build PROPERTY FOLDER "lanes")

  # Deletion buttons, all the same mechanism (cmake -E rm -rf of a directory this graph owns):
  # one per cache group, one for the root. The group dirs are the units cargo itself works in,
  # so a group clean is exactly "drop that feature config's artifacts".
  foreach(_group stub live)
    polyorch_rust_clean(NAME talkservo-lanes-${_group}-clean
        BASE_DIR "${TALKSERVO_CACHE_ROOT}/${_group}" FOLDER "lanes")
  endforeach()
  polyorch_rust_clean(NAME talkservo-lanes-clean BASE_DIR "${TALKSERVO_CACHE_ROOT}" FOLDER "lanes")
endfunction()

# talkservo_assert_target_folders() - call LAST (after web/gates/install) from CMakeLists.
# Rationale: FOLDER has no effect under Ninja, so a missing or misspelled value is invisible
# until someone opens the graph in an IDE - and the only reason that is worth enforcing here is
# that this project's verbs are meant to be findable by name. Same stance as the gate-clippy
# argv assertion in 5e3c91e: if a name claims a family, check the target is in it.
function(talkservo_assert_target_folders)
  get_property(_targets DIRECTORY PROPERTY BUILDSYSTEM_TARGETS)
  set(_bad "")
  foreach(_t ${_targets})
    if(_t MATCHES "^talkservo-")
      get_property(_f TARGET ${_t} PROPERTY FOLDER)
      if(NOT _f)
        list(APPEND _bad ${_t})
      endif()
      if(TALKSERVO_FOLDERS_REPORT)
        message(STATUS "folder: ${_t} -> ${_f}")
      endif()
    endif()
  endforeach()
  if(_bad)
    message(FATAL_ERROR "targets with no IDE FOLDER (use set_property, add_custom_target has no "
                        "FOLDER keyword): ${_bad}")
  endif()
endfunction()
