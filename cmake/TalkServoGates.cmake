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

  _polyorch_rust_command(_cmd SUBCOMMAND fmt --all -- --check ENV "${_fenv}")
  add_test(NAME gate-fmt COMMAND ${_cmd} WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(gate-fmt PROPERTIES LABELS "gate;headless" TIMEOUT 120)

  _polyorch_rust_command(_cmd SUBCOMMAND clippy --workspace --all-targets -- -D warnings ENV "${_fenv}")
  add_test(NAME gate-clippy COMMAND ${_cmd} WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(gate-clippy PROPERTIES LABELS "gate;headless;slow" TIMEOUT 900)

  # cargo-deny is an OPTIONAL tool (absent from the conda env today; the
  # `audit` task has the same latent gap) - runtime probe, honest skip.
  get_filename_component(_cargodir "${POLYORCH_RUST_CARGO}" DIRECTORY)
  add_test(NAME gate-deny
      COMMAND sh -c "export PATH='${_cargodir}:\$PATH'; command -v cargo-deny >/dev/null 2>&1 || exit 77; exec cargo-deny check"
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
  message(STATUS "talkservo gates: gate-docs/fmt/clippy/deny + e2e-stub/e2e-live registered")
endfunction()
