# TalkServoGates.cmake — gate face + e2e labels in ctest (plan-8 P3).
# gate-docs runs scripts/gate.sh (ledger blocks are verbatim from conventions.md);
# fmt/clippy ride the SAME _polyorch_rust_command wrapper as the lane test bridges
# (pixi prefix + host-leak strip -> button, lane and gate can never drift in env).
# e2e: stub lane default, live lane env-gated at RUN time (exit 77 = ctest skip),
# TS_E2E_FEATURES/TS_COTURN semantics unchanged.

option(TALKSERVO_GATES "register gate + e2e entries in ctest" ON)

# talkservo_gates() - call AFTER talkservo_ensure_polyorch_rust().
function(talkservo_gates)
  if(NOT TALKSERVO_GATES)
    return()
  endif()
  if(NOT COMMAND polyorch_rust_test OR NOT POLYORCH_RUST_FOUND)
    message(STATUS "talkservo gates: rust face not available -- off")
    return()
  endif()

  _polyorch_rust_forward_env(TRIPLE "${POLYORCH_RUST_HOST_TARGET}" OUT_ENV _fenv)

  # --- gate face ---
  add_test(NAME gate-docs
      COMMAND bash "${CMAKE_SOURCE_DIR}/scripts/gate.sh"
      WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(gate-docs PROPERTIES LABELS "gate;headless" TIMEOUT 120)

  _polyorch_rust_command(_fmt_cmd SUBCOMMAND fmt --all -- --check ENV "${_fenv}")
  add_test(NAME gate-fmt COMMAND ${_fmt_cmd} WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(gate-fmt PROPERTIES LABELS "gate;headless" TIMEOUT 120)

  # plan-10 T6, measured and REVERTED to cargo's default ./target: pointing clippy at the
  # live cache group looked like free dedup (one workspace, two target dirs, ./target once
  # measured 33 GB) but is not. `cargo clean -p mediasoup-sys` + the exact gate command in
  # out/cargo-target/live did not finish in 23 minutes - the C++ worker rebuild lands inside
  # the clippy path either way, so sharing bought nothing and only coupled the gate to the
  # lane cache. The 33 GB was clippy's own rmeta set, not accidental duplication. Cold cost
  # stays: warm gate-clippy measured 35.7s, cold runs tens of minutes in ANY directory.
  # Env parity with the lanes, WITHOUT sharing their cache: mediasoup-sys' build script
  # fetches its openssl subproject through meson, and under this host's MITM proxy that
  # download fails as "Subproject openssl is buildable: NO / has no meson.build" (measured
  # 2026-10-22, cold gate-clippy). The live lanes pass because _laneenv carries NO_PROXY;
  # the gate is the same build graph with a different environment, so it must carry the
  # same bypass. The linker pin comes along for identity, not necessity (clippy links
  # nothing) - a gate that diverges from the lanes in env fails in a way the lanes do not.
  set(_clippy_env "NO_PROXY=*" "no_proxy=*")
  get_filename_component(_cargodir "${POLYORCH_RUST_CARGO}" DIRECTORY)
  if(DEFINED TALKSERVO_CONDA_CC)
    string(TOUPPER "${POLYORCH_RUST_HOST_TARGET}" _clinktr)
    string(REPLACE "-" "_" _clinktr "${_clinktr}")
    list(APPEND _clippy_env "CARGO_TARGET_${_clinktr}_LINKER=${TALKSERVO_CONDA_CC}")
  endif()
  _polyorch_rust_command(_clippy_cmd SUBCOMMAND clippy --workspace --all-targets -- -D warnings ENV ${_fenv} ${_clippy_env})
  # configure-time assertion, not a wish: the c1334f9 cleanup deleted this command line and
  # gate-clippy silently inherited gate-fmt's ${_cmd} - a push gate that "passed" by running
  # `cargo fmt --check`. A stale variable is invisible to the gate itself, so the graph checks
  # that the thing named clippy actually carries clippy.
  list(FIND _clippy_cmd "clippy" _clippy_at)
  if(_clippy_at LESS_EQUAL -1)
    message(FATAL_ERROR "gate-clippy command does not invoke clippy: ${_clippy_cmd}")
  endif()
  add_test(NAME gate-clippy COMMAND ${_clippy_cmd} WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(gate-clippy PROPERTIES LABELS "gate;headless;slow" TIMEOUT 900)

  # cargo-deny is an OPTIONAL tool - the skip below stays honest when it is truly
  # absent. But it is installed as a USER cargo subcommand ($CARGO_HOME/bin, default
  # ~/.cargo/bin), which neither the pixi env nor ctest puts on PATH: the probe looked
  # for a binary it could not see, so every run reported the supply-chain check as
  # Skipped while the tool sat on the disk. Widening PATH costs nothing and the
  # `audit` task gets the same treatment.
  # /usr/bin:/bin are NOT optional here: cargo-deny shells out to `git` to update the
  # RustSec DB, and the shell `pixi run` spawns starts with a PATH so thin that even
  # /usr/bin is missing (measured: `command -v git` empty, `tail` not found) - the run
  # then dies as "failed to spawn git: No such file or directory", which reads like a
  # tooling bug and is really a missing PATH entry.
  get_filename_component(_cargodir "${POLYORCH_RUST_CARGO}" DIRECTORY)
  set(_cargo_home "$ENV{CARGO_HOME}")
  if(_cargo_home STREQUAL "")
    set(_cargo_home "$ENV{HOME}/.cargo")
  endif()
  add_test(NAME gate-deny
      COMMAND sh -c "export PATH='${_cargo_home}/bin:${_cargodir}:/usr/bin:/bin:\$PATH'; command -v cargo-deny >/dev/null 2>&1 || exit 77; exec cargo-deny check"
      WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(gate-deny PROPERTIES LABELS "gate;headless" TIMEOUT 300 SKIP_RETURN_CODE 77)

  # --- e2e face ---
  # TS_ADMIN=1 belongs HERE too: without it the whole admin spec self-skips and the
  # gate reports green while never exercising the console, the audit tab or the
  # registry lock path. The harness already mints the same fixed admin secret.
  add_test(NAME e2e-stub
      COMMAND sh -c "cd '${CMAKE_SOURCE_DIR}/web' && TS_ADMIN=1 npx playwright test"
      WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(e2e-stub PROPERTIES LABELS "e2e-stub;headless" TIMEOUT 900)

  add_test(NAME e2e-live
      COMMAND sh -c "test \"\${TS_E2E_FEATURES:-}\" = live || exit 77; cd '${CMAKE_SOURCE_DIR}/web' && TS_E2E_FEATURES=live npx playwright test"
      WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(e2e-live PROPERTIES LABELS "e2e-live" TIMEOUT 1800 SKIP_RETURN_CODE 77)
  # IDE-facing aggregate verbs: these are deliberately not ALL. They make the
  # gate/e2e graph surface clickable without turning `build all` into a test run.
  add_custom_target(talkservo-gates-build
      COMMAND ${CMAKE_CTEST_COMMAND}
              --test-dir "${CMAKE_BINARY_DIR}"
              -L gate
              --output-on-failure
      COMMENT "run ctest -L gate")
  set_property(TARGET talkservo-gates-build PROPERTY FOLDER "lanes")

  add_custom_target(talkservo-e2e-stub-build
      DEPENDS talkservo-lanes-stub-build
      COMMAND ${CMAKE_CTEST_COMMAND}
              --test-dir "${CMAKE_BINARY_DIR}"
              -L e2e-stub
              --output-on-failure
      COMMENT "run ctest -L e2e-stub")
  set_property(TARGET talkservo-e2e-stub-build PROPERTY FOLDER "lanes")

  message(STATUS "talkservo gates: gate-docs/fmt/clippy/deny + e2e-stub/e2e-live + IDE gate/e2e targets registered")
endfunction()
