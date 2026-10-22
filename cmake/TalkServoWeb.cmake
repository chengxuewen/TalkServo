# TalkServoWeb.cmake — Node lane for the SPA (plan-8 P2). PolyOrch Node helpers drive
# npm scripts; vite/tsc/vitest keep their package.json authority (red line: no rule
# duplication). ctest bridge runs `npm run test` directly (vitest) with label "web".

option(TALKSERVO_WEB "register the web Node lane in the graph" ON)

# talkservo_web() - call AFTER talkservo_ensure_polyorch() (node setup runs here).
function(talkservo_web)
  if(NOT TALKSERVO_WEB)
    return()
  endif()
  if(NOT COMMAND polyorch_node_setup)
    message(FATAL_ERROR "call talkservo_ensure_polyorch() first - PolyOrch helpers absent")
  endif()
  polyorch_node_setup(QUIET)
  # Provisioning is NOT the graph's job (polyorch_node_build only runs `npm run build`; the
  # install lives in the pixi task `web-install` = `npm ci --prefix web`, per plan-8's line).
  # Say so at configure time: the raw symptom for a fresh clone is `sh: 1: tsc: not found`
  # buried in npm output, which reads like a broken toolchain (found by the plan-10 drill).
  foreach(_pkg web packages/client)
    if(NOT EXISTS "${CMAKE_SOURCE_DIR}/${_pkg}/node_modules")
      if(_pkg STREQUAL "web")
        set(_provision_task web-install)
      else()
        set(_provision_task client-install)
      endif()
      message(WARNING "${_pkg}/node_modules is missing - run pixi run ${_provision_task} once per "
                      "clone before the web lane, talkservo-lanes-stub-build or ctest -L rust|web can "
                      "succeed: rust-embed needs web/dist, vite needs the web deps, and tsc --noEmit "
                      "typechecks ../packages/client/src so the SDK needs its own deps installed.")
    endif()
  endforeach()
  if(NOT POLYORCH_NODE_FOUND)
    message(STATUS "talkservo web: node toolchain absent -- lane off (skeleton CI stays rust-only)")
    return()
  endif()

  set(_mf "${CMAKE_SOURCE_DIR}/web/package.json")
  polyorch_node_build(TARGET talkservo-web MANIFEST "${_mf}")

  # Lane ordering: embedded-web cargo lanes bake web/dist at COMPILE time (rust-embed);
  # the web lane must run first or fresh clones die with a confusing E0599 from Assets.
  # The list comes from the lane registry (plan-10 T2). It used to be a hand-written name
  # list, which is the silent kind of bug: renaming a lane drops the ordering edge and
  # the symptom is a binary serving stale assets, not a build error.
  get_property(_ew_handles GLOBAL PROPERTY TALKSERVO_EMBEDDED_WEB_HANDLES)
  foreach(_h ${_ew_handles})
    if(TARGET ${_h}-build AND TARGET talkservo-web-build)
      add_dependencies(${_h}-build talkservo-web-build)
    endif()
  endforeach()
  if(TARGET talkservo-lanes-stub-build AND TARGET talkservo-web-build)
    add_dependencies(talkservo-lanes-stub-build talkservo-web-build)
  endif()

  if(TALKSERVO_WEB AND COMMAND polyorch_node_test)
    polyorch_node_test(TARGET talkservo-web)  # -test button verb
  endif()

  # CTest fixture: the embedded-web lanes compile rust-embed against web/dist, so the SPA
  # build must run before those ctest entries even when nobody ran a build step first (the
  # plan-10 fresh-clone drill found the graph relying on an already-warm tree).
  add_test(NAME web-dist-fixture
           COMMAND ${CMAKE_COMMAND} --build "${CMAKE_BINARY_DIR}" --target talkservo-web-build
           WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(web-dist-fixture PROPERTIES
      FIXTURES_SETUP webdist TIMEOUT 300 LABELS "fixture")
  get_property(_need_webdist GLOBAL PROPERTY TALKSERVO_WEBDIST_TESTS)
  foreach(_t ${_need_webdist})
    set_tests_properties(${_t} PROPERTIES FIXTURES_REQUIRED webdist)
  endforeach()

  # ctest bridge (same script authority as the button: package.json "test" = vitest run).
  add_test(NAME web-test
           COMMAND npm --prefix "${CMAKE_SOURCE_DIR}/web" run test
           WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(web-test PROPERTIES
      LABELS "web;headless"
      FIXTURES_REQUIRED webdist
      TIMEOUT 180)
  message(STATUS "talkservo web: node lane registered (talkservo-web build/test)")

  # web/dist is the input the embedded-web lanes bake, so clearing it is the one
  # Node-side cleanup the graph owns. node_modules stays npm's business (plan-8 red line).
  add_custom_target(talkservo-web-clean
      COMMAND ${CMAKE_COMMAND} -E rm -rf "${CMAKE_SOURCE_DIR}/web/dist"
      COMMENT "removing web/dist (the embedded-web input)")
  # One group for the whole web object: the Node helper only sets FOLDER when
  # PolyOrch_RUST_FOLDER_ROOT is set, so its two targets are pinned here explicitly -
  # otherwise web-build/web-test sit at the IDE root while web-clean is in "web".
  foreach(_t talkservo-web-build talkservo-web-test talkservo-web-clean)
    if(TARGET ${_t})
      set_property(TARGET ${_t} PROPERTY FOLDER "web")
    endif()
  endforeach()
endfunction()
