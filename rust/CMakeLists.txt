message(STATUS "Adding rust libraries")

# Use generated rust files because protobuf compiler is unavailable yet.
add_subdirectory(netsim-cxx)
# Steps to generate rust files are as follows:
#
# 1. sudo apt-get install protobuf-compiler
# 2. Uncomment `build = "build_cargo.rs"` in rust/netsim-cxx/Cargo.toml.
# 3. cd rust/netsim-cxx && cxxbridge.sh
# 4. Comment `build = "build_cargo.rs"` in rust/netsim-cxx/Cargo.toml.

add_subdirectory(netsim-cli)
add_subdirectory(frontend-client-cxx)
add_subdirectory(frontend-proto)
# Steps to generate rust files are as follows:
#
# 1. sudo apt-get install protobuf-compiler
# 2. Uncomment `build = "build_cargo.rs"` in rust/frontend-proto/Cargo.toml.
# 3. cargo build
# 4. bash scripts/format_code.sh

add_library(netsim-rust-lib INTERFACE)
target_link_libraries(netsim-rust-lib INTERFACE netsim-cxx netsim-cxx-generated
                                                frontend-proto)

android_license(TARGET "netsim-cxx" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "backend-proto" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "netsim-rust-lib" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "frontend-proto" LIBNAME None SPDX None LICENSE None
                LOCAL None)
android_license(TARGET "frontend-client-cxx" LIBNAME None SPDX None LICENSE None
                LOCAL None)

# For netsim rust libraries development.
android_add_executable(TARGET netsim-rust-dev LICENSE Apache-2.0 SRC main.cc
                       DEPS netsim-rust-lib)

add_library(netsim-cli-rust-lib INTERFACE)

target_link_libraries(netsim-cli-rust-lib
                      INTERFACE frontend-proto frontend-client-cxx netsim-cli)

android_license(TARGET netsim-cli-rust-lib LIBNAME None SPDX None LICENSE None
                LOCAL None)

android_add_executable(
  TARGET netsim-cl
  LICENSE Apache-2.0 INSTALL .
  SRC netsim-cl.cc
  DEPS frontend-client
       frontend-grpc-lib
       frontend_client_cxx_generated
       grpc++
       netsim-cli-rust-lib
       protobuf::libprotobuf
       protobuf::libprotobuf
       util-lib)
