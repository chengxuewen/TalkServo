# TalkServo — assembly layer (plan-8, D25). Cargo keeps workspace authority;
# this graph DRIVES it through the PolyOrch adapter surface. Five-part order modeled
# on the ModuKit D66 / VisiaEngine D39 consumer shape; runtime verbs (oxmgr facade,
# compose) are NOT graph members (plan-8 red line 1).

cmake_minimum_required(VERSION 3.25)   # PolyOrch embed floor (D39/D66 equivalent)

# Version single source: the workspace Cargo.toml (no literal copies — mismatch is
# a configure error; ModuKit D67 grammar).
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" _ws)
string(REGEX MATCH "version = \"([0-9]+\\.[0-9]+\\.[0-9]+)\"" _m "${_ws}")
if(NOT _m)
  message(FATAL_ERROR "Failed to read the Cargo.toml workspace version (single-source contract broken)")
endif()

project(talkservo VERSION ${CMAKE_MATCH_1} LANGUAGES CXX)  # CXX identity required before the
                                                            # PolyOrch embed (PlatformSupport
                                                            # contract, ModuKit first-red M1
                                                            # observed lesson)

enable_testing()                        # must run before ALL add_subdirectory calls (measured lesson)

include(cmake/TalkServoPolyOrch.cmake)
talkservo_ensure_polyorch()             # sentinel embed: outer scope first-wins, else gitlink pin
talkservo_ensure_polyorch_rust()        # TALKSERVO_USE_PIXI -> polyorch_rust_setup(FROM pixi)

include(cmake/TalkServoCrates.cmake)
talkservo_crates()                      # P2: core+sfu+server stub lanes -> ctest "rust"; mediasoup =
                                        # build-only artifact handle (test verb stays test-sfu until P3)
include(cmake/TalkServoWeb.cmake)
talkservo_web()                         # P2: web Node lane (npm scripts keep authority) -> ctest "web"
include(cmake/TalkServoGates.cmake)
talkservo_gates()                       # P3: gate-docs/fmt/clippy/deny + e2e-stub/live ctest labels

include(cmake/TalkServoInstall.cmake)
talkservo_install_tree()
talkservo_assert_target_folders()   # plan-10 ratchet: every talkservo-* verb declares its IDE group                # P4: components bin (server+oxmgr) / dist (etc+scripts)
# add_subdirectory(crates/...) stays off — lanes ride the helper surface; cargo keeps
# its workspace authority (D65 equivalence, plan-8 red line 2: mediasoup worker build.rs-own).

# Reserved wiring slots (lit per plan-8 phases, NOT speculative scaffolding):
#   P4: install tree
