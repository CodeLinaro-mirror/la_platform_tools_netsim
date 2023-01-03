message(STATUS "Adding rust ${CMAKE_CURRENT_LIST_DIR}/ranging/Cargo.toml")

add_subdirectory(ranging)
add_subdirectory(netsim-cli)
add_subdirectory(frontend-proto)
# Use generated rust files because protobuf compiler is unavailable yet. Steps
# to generate rust files are as follows:

# 1. sudo apt-get install protobuf-compiler
# 2. Uncomment `build = "build_cargo.rs"` in rust/frontend-proto/Cargo.toml.
# 3. cargo build
# 4. bash scripts/format_code.sh

add_library(netsim-rust-lib INTERFACE)
target_link_libraries(netsim-rust-lib INTERFACE ranging ranging_cxx
                                                frontend-proto)

android_license(TARGET "netsim-rust-lib" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "ranging" LIBNAME None SPDX None LICENSE None LOCAL None)
android_license(TARGET "frontend-proto" LIBNAME None SPDX None LICENSE None
                LOCAL None)

# For netsim rust libraries development.
android_add_executable(TARGET netsim-rust-dev LICENSE Apache-2.0 INSTALL .
                       SRC main.cc DEPS netsim-rust-lib)
