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

  # lane ordering: embedded-web cargo lanes bake web/dist at COMPILE time
  # (rust-embed); the web lane must run first or fresh clones die with
  # confusing E0599 from Assets. Guards: mediator targets exist per lane.
  foreach(_dep talkservo-server talkservo-server-bin talkservo-server-mediasoup)
    if(TARGET ${_dep}-build AND TARGET talkservo-web-build)
      add_dependencies(${_dep}-build talkservo-web-build)
    endif()
  endforeach()

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
endfunction()
