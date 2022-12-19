message(STATUS "Adding rust ${CMAKE_CURRENT_LIST_DIR}/ranging/Cargo.toml")

add_subdirectory(ranging)

add_library(netsim-rust-lib INTERFACE)
target_link_libraries(netsim-rust-lib INTERFACE ranging ranging_cxx)

android_license(TARGET "netsim-rust-lib" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "ranging" LIBNAME None SPDX None LICENSE None LOCAL None)
