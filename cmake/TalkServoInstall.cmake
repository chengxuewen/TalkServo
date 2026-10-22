# TalkServoInstall.cmake — install tree (plan-8 P4). Stages the shapes the shell
# verbs always produced, as CMake components:
#   --component bin   <prefix>/bin/{talkservo-server, oxmgr}     (cmd_build tree)
#   --component dist  <prefix>/{etc,scripts}/*                   (cmd_package tar)
# `cmake --install` full (no -C) = bin+dist = the package staging tree.
# tar naming/top-dir contract stays the script's (it knows --dist/--lane/ver);
# web/dist copy stays the script's too (ops-side passthrough, not a build).

set(TALKSERVO_PACKAGE_LANE "live" CACHE STRING "lane whose server artifact ships (stub|live)")
set(TALKSERVO_OXMGR "" CACHE FILEPATH "staged oxmgr binary (empty = WARN-and-skip, script semantics)")

# talkservo_install_tree() - call AFTER talkservo_gates().
function(talkservo_install_tree)
  if(NOT TALKSERVO_PACKAGE_LANE MATCHES "^(stub|live)$")
    message(FATAL_ERROR "TALKSERVO_PACKAGE_LANE must be stub or live, got '${TALKSERVO_PACKAGE_LANE}'")
  endif()
  # The handle name is composed from the lane word (plan-10 T2) - no if/else name table
  # to keep in sync: stub -> talkservo-server-stub, live -> talkservo-server-live.
  set(_handle talkservo-server-${TALKSERVO_PACKAGE_LANE})

  polyorch_rust_install(
      TARGETS ${_handle}
      RUNTIME_DESTINATION bin
      COMPONENT bin
      PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE GROUP_READ GROUP_EXECUTE WORLD_READ WORLD_EXECUTE)

  if(TALKSERVO_OXMGR)
    install(PROGRAMS "${TALKSERVO_OXMGR}" DESTINATION bin RENAME oxmgr COMPONENT bin)
  else()
    message(WARNING "TALKSERVO_OXMGR empty - oxmgr will not ship (start/stop fall back to PATH)")
  endif()

  # dist component: the package's etc/ + scripts/ (existence-guarded = the
  # script's `cp ... || true` semantics preserved)
  if(EXISTS "${CMAKE_SOURCE_DIR}/config/sample.env")
    install(FILES "${CMAKE_SOURCE_DIR}/config/sample.env" DESTINATION etc COMPONENT dist)
  endif()
  foreach(_f deploy/Caddyfile deploy/talkservo.service)
    if(EXISTS "${CMAKE_SOURCE_DIR}/${_f}")
      install(FILES "${CMAKE_SOURCE_DIR}/${_f}" DESTINATION etc COMPONENT dist)
    endif()
  endforeach()
  if(EXISTS "${CMAKE_SOURCE_DIR}/3rdparty/oxmgr/README.md")
    install(FILES "${CMAKE_SOURCE_DIR}/3rdparty/oxmgr/README.md"
            DESTINATION etc RENAME oxmgr-README.md COMPONENT dist)
  endif()
  install(PROGRAMS "${CMAKE_SOURCE_DIR}/scripts/issue-token.sh" DESTINATION scripts COMPONENT dist)
  message(STATUS "talkservo install: lane=${TALKSERVO_PACKAGE_LANE} ships ${_handle}; oxmgr=${TALKSERVO_OXMGR}")
endfunction()
