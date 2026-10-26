# TalkServoInstall.cmake — install tree (plan-8 P4). Stages the shapes the shell
# verbs always produced, as CMake components:
#   --component bin   <prefix>/bin/{talkservo-server, oxmgr}     (cmd_build tree)
#   --component dist  <prefix>/{etc,scripts}/*                   (cmd_package tar)
# `cmake --install` full (no -C) = bin+dist = the package staging tree.
# tar naming/top-dir contract stays the script's (it knows --dist/--lane/ver);
# web/dist copy stays the script's too (ops-side passthrough, not a build).

set(TALKSERVO_PACKAGE_LANE "live" CACHE STRING "lane whose server artifact ships (stub|live)")
set(TALKSERVO_OXMGR "" CACHE FILEPATH "staged oxmgr binary (empty = WARN-and-skip, script semantics)")
set(TALKSERVO_OXMGR_VERSION "v0.5.0" CACHE STRING "vendored oxmgr version tag (talkservo.sh OXMGR_VER; drives the .cache/oxmgr staging copy)")

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

# talkservo_deploy_targets() - call AFTER talkservo_install_tree(). One staging
# button per lane (build:deploy minus the runtime `init` — D25 red line keeps
# instance lifecycle out of the graph): builds that lane's server in THIS build
# tree's config and stages bin + vendored oxmgr into out/<lane>/bin. Deliberately
# NOT `cmake --install`: the install handle is fixed per configure by
# TALKSERVO_PACKAGE_LANE, while these buttons must work for either lane from one
# configure. web/dist stays baked into the binary (rust-embed); the ops copy
# remains the shell's passthrough (same split as plan-8 P4).
function(talkservo_deploy_targets)
  foreach(_lane stub live)
    set(_handle talkservo-server-${_lane})
    if(NOT TARGET ${_handle})
      message(FATAL_ERROR "talkservo deploy: lane handle ${_handle} not registered (crates face order changed?)")
    endif()
    set(_dest "${CMAKE_SOURCE_DIR}/out/${_lane}/bin")
    file(GLOB _oxmgr_candidates "${CMAKE_SOURCE_DIR}/.cache/oxmgr/oxmgr-${TALKSERVO_OXMGR_VERSION}-*/oxmgr")
    set(_copy_oxmgr "")
    if(_oxmgr_candidates)
      list(GET _oxmgr_candidates 0 _oxmgr_cache)
      set(_copy_oxmgr
        COMMAND ${CMAKE_COMMAND} -E copy "${_oxmgr_cache}" "${_dest}/oxmgr")
    endif()
    add_custom_target(talkservo-deploy-${_lane}-build
        DEPENDS ${_handle}-build
        COMMAND ${CMAKE_COMMAND} -E make_directory "${_dest}"
        COMMAND ${CMAKE_COMMAND} -E copy
                "$<TARGET_FILE:${_handle}>" "${_dest}/talkservo-server"
        ${_copy_oxmgr}
        COMMENT "stage ${_handle} -> out/${_lane}/bin (deploy staging; init stays in talkservo.sh)")
    set_property(TARGET talkservo-deploy-${_lane}-build PROPERTY FOLDER "lanes")
  endforeach()
  message(STATUS "talkservo deploy: staging buttons talkservo-deploy-{stub,live}-build -> out/<lane>/bin")
endfunction()
