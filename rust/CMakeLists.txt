message(STATUS "Adding rust libraries")

add_subdirectory(backend-cxx)
add_subdirectory(ranging)
add_subdirectory(netsim-cli)
add_subdirectory(frontend-client-cxx)
add_subdirectory(frontend-proto)
# Use generated rust files because protobuf compiler is unavailable yet. Steps
# to generate rust files are as follows:
#
# 1. sudo apt-get install protobuf-compiler
# 2. Uncomment `build = "build_cargo.rs"` in rust/frontend-proto/Cargo.toml.
# 3. cargo build
# 4. bash scripts/format_code.sh

add_subdirectory(http-server)

add_library(netsim-rust-lib INTERFACE)
target_link_libraries(
  netsim-rust-lib INTERFACE backend-cxx backend-cxx-generated frontend-proto
                            ranging ranging_cxx)

android_license(TARGET "backend-cxx" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "backend-proto" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "netsim-rust-lib" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "ranging" LIBNAME None SPDX None LICENSE None LOCAL None)
android_license(TARGET "frontend-proto" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "frontend-client-cxx" LIBNAME None SPDX None LICENSE None
                LOCAL None)

# For netsim rust libraries development.
android_add_executable(TARGET netsim-rust-dev LICENSE Apache-2.0 INSTALL .
                       SRC main.cc DEPS netsim-rust-lib)
