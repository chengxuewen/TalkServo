# OxmgrVendorScript.cmake — `cmake -P` body of talkservo-oxmgr-vendor.
# Idempotent: unpacks OX_ARCHIVE into OX_DEST_DIR/oxmgr only when missing; otherwise the
# verify-then-unpack discipline (sha256 against the sidecar, refuse on mismatch) mirrors
# talkservo.sh ensure_oxmgr, so an already-cached binary short-circuits with no re-hash.

if(EXISTS "${OX_DEST_DIR}/oxmgr")
  message(STATUS "oxmgr vendor: already unpacked at ${OX_DEST_DIR}/oxmgr")
  return()
endif()

file(SHA256 "${OX_ARCHIVE}" _got)
file(STRINGS "${OX_SIDECAR}" _sidecar_lines LIMIT_COUNT 1)
string(REGEX MATCH "^[0-9a-fA-F]+" _want "${_sidecar_lines}")
string(LENGTH "${_want}" _want_len)
if(NOT _want_len EQUAL 64)
  message(FATAL_ERROR "malformed sidecar (no 64-hex digest at line start): ${OX_SIDECAR}")
endif()

if(NOT _want STREQUAL _got)
  message(FATAL_ERROR
    "oxmgr archive checksum mismatch — refuse to unpack\n"
    "  archive : ${OX_ARCHIVE}\n"
    "  sha256  : ${_got}\n"
    "  expected: ${_want}  (${OX_SIDECAR})")
endif()

file(MAKE_DIRECTORY "${OX_DEST_DIR}")
execute_process(
  COMMAND ${CMAKE_COMMAND} -E tar xzf "${OX_ARCHIVE}"
  WORKING_DIRECTORY "${OX_DEST_DIR}"
  RESULT_VARIABLE _rc)
if(NOT _rc EQUAL 0)
  message(FATAL_ERROR "oxmgr unpack failed (exit ${_rc}): ${OX_ARCHIVE}")
endif()

# the tarball's top entry is `oxmgr` itself
if(NOT EXISTS "${OX_DEST_DIR}/oxmgr")
  message(FATAL_ERROR "oxmgr unpacked but ${OX_DEST_DIR}/oxmgr is absent — archive layout changed")
endif()
file(CHMOD "${OX_DEST_DIR}/oxmgr" PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE
    GROUP_READ GROUP_EXECUTE WORLD_READ WORLD_EXECUTE)
message(STATUS "oxmgr vendor: unpacked ${OX_ARCHIVE} -> ${OX_DEST_DIR}/oxmgr")
