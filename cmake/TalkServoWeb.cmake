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

  # ctest bridge (same script authority as the button: package.json "test" = vitest run).
  add_test(NAME web-test
           COMMAND npm --prefix "${CMAKE_SOURCE_DIR}/web" run test
           WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
  set_tests_properties(web-test PROPERTIES
      LABELS "web;headless"
      TIMEOUT 180)
  message(STATUS "talkservo web: node lane registered (talkservo-web build/test)")

  # web/dist is the input the embedded-web lanes bake, so clearing it is the one
  # Node-side cleanup the graph owns. node_modules stays npm's business (plan-8 red line).
  add_custom_target(talkservo-web-clean
      COMMAND ${CMAKE_COMMAND} -E rm -rf "${CMAKE_SOURCE_DIR}/web/dist"
      COMMENT "removing web/dist (the embedded-web input)"
      FOLDER "web")
endfunction()
