# TalkServoOxmgrVendor.cmake — build-time vendoring of the oxmgr supervisor binary
# (deploy bin/ completeness, 2026-10-27).
#
# Why: talkservo-deploy-build used file(GLOB) on .cache/oxmgr — a CONFIGURE-time snapshot.
# A tree configured before .cache/oxmgr existed (fresh clone → IDE button) froze an empty
# copy list and silently staged a half bin/ (server without oxmgr; start then fails on
# "oxmgr not found"). The glob cannot re-run at build time (CONFIGURE_DEPENDS does not
# apply to file(GLOB)), so the decision moves to the build: a deterministic, arch-derived
# cache path plus a custom target whose -P script verifies-then-unpacks the committed
# 3rdparty archive, idempotent when already unpacked.
#
# Red-line compliance (D25): unpacking a committed archive is a BUILD act; the graph never
# RUNS oxmgr and never touches instance layout (init stays in talkservo-server itself).
# talkservo.sh ensure_oxmgr keeps its own copy of the scheme (same archives, same
# .cache layout, same refuse-to-unpack discipline) — the shell flow never requires cmake.

# talkservo_oxmgr_vendor(OUT <var>)
# On Linux x86_64/aarch64: registers target talkservo-oxmgr-vendor, sets <var> to the
# deterministic extracted path .cache/oxmgr/oxmgr-<ver>-<triple>/oxmgr.
# Other hosts/arches: <var> empty + WARNING (mirrors the shell's WARN-on-unknown-arch
# posture — PATH fallback stays the user's option, never a wrong-arch binary).
# capture the defining file location NOW: inside a function body, CMAKE_CURRENT_LIST_DIR
# resolves against the CALLER's file (measured 2026-10-27: the -P path landed at the repo root)
set(TALKSERVO_OXMGR_VENDOR_SCRIPT "${CMAKE_CURRENT_LIST_DIR}/OxmgrVendorScript.cmake")

function(talkservo_oxmgr_vendor)
  cmake_parse_arguments(PARSE_ARGV 0 V "" "OUT" "")
  if(NOT V_OUT)
    message(FATAL_ERROR "talkservo_oxmgr_vendor: OUT is required")
  endif()

  set(_triple "")
  if(CMAKE_HOST_SYSTEM_NAME STREQUAL "Linux")
    string(TOLOWER "${CMAKE_HOST_SYSTEM_PROCESSOR}" _proc)
    if(_proc STREQUAL "x86_64" OR _proc MATCHES "^amd64")
      set(_triple "x86_64-unknown-linux-musl")
    elseif(_proc MATCHES "^(aarch64|arm64)$")
      set(_triple "aarch64-unknown-linux-musl")
    endif()
  endif()

  if(_triple STREQUAL "")
    message(WARNING "talkservo oxmgr vendor: no vendored oxmgr for "
        "${CMAKE_HOST_SYSTEM_NAME}/${CMAKE_HOST_SYSTEM_PROCESSOR} - deploy bin/ ships "
        "talkservo-server only; put oxmgr on PATH or use a supported Linux arch")
    set(${V_OUT} "" PARENT_SCOPE)
    return()
  endif()

  set(_pkg "oxmgr-${TALKSERVO_OXMGR_VERSION}-${_triple}")
  set(_archive "${CMAKE_SOURCE_DIR}/3rdparty/oxmgr/${_pkg}.tar.gz")
  set(_sidecar "${_archive}.sha256")
  set(_dest "${CMAKE_SOURCE_DIR}/.cache/oxmgr/${_pkg}")

  # committed-archive integrity is a checkout fact, not a build-time surprise
  if(NOT EXISTS "${_archive}" OR NOT EXISTS "${_sidecar}")
    message(FATAL_ERROR "talkservo oxmgr vendor: ${_pkg} archive/sidecar missing under "
        "3rdparty/oxmgr/ - the vendored supervisor is committed repo data, restore it")
  endif()

  if(NOT TARGET talkservo-oxmgr-vendor)  # called by BOTH install_tree and deploy_targets
  add_custom_target(talkservo-oxmgr-vendor
    COMMAND ${CMAKE_COMMAND}
        -DOX_ARCHIVE=${_archive}
        -DOX_SIDECAR=${_sidecar}
        -DOX_DEST_DIR=${_dest}
        -P "${TALKSERVO_OXMGR_VENDOR_SCRIPT}"
    COMMENT "vendor oxmgr ${TALKSERVO_OXMGR_VERSION} (${_triple}) -> .cache/oxmgr"
    VERBATIM)
  set_property(TARGET talkservo-oxmgr-vendor PROPERTY FOLDER "lanes")
  endif()

  set(${V_OUT} "${_dest}/oxmgr" PARENT_SCOPE)
endfunction()
