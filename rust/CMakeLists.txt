message(STATUS "Adding rust ${CMAKE_CURRENT_LIST_DIR}/Cargo.toml")

# This will automatically register all the tests as well.
corrosion_import_crate(MANIFEST_PATH Cargo.toml FLAGS --offline --verbose)
corrosion_set_env_vars(netsim-cxx CARGO_HOME=${Rust_CARGO_HOME})
corrosion_set_env_vars(netsim-cli CARGO_HOME=${Rust_CARGO_HOME})
corrosion_set_env_vars(frontend-proto CARGO_HOME=${Rust_CARGO_HOME})
corrosion_set_env_vars(frontend-client-cxx CARGO_HOME=${Rust_CARGO_HOME})

# netsim-cxx
if(WIN32)
  # 1. Set RUST_CXX_NO_EXCEPTIONS preprocesor definition for cxx crate to disable
  #    compilation error message of "cannot use 'throw' with exceptions".
  # 2. MSVC uses environment variable "CL" to prepend arguments to the
  #    command-line arguments. However, we verified MSVC also supports CXXFLAGS,
  #    which is used by cc crate.
  # 3. This isn't needed in mac and Linux because corrosion doesn't pass
  #    -fno-exception flag.
  corrosion_set_env_vars(netsim-cxx CXXFLAGS=/DRUST_CXX_NO_EXCEPTIONS)
endif()

if(LINUX)
  # b/260271048
  corrosion_add_target_rustflags(
    "netsim-cxx" "-Clink-args=-Wl,-rpath,${CMAKE_LIBRARY_OUTPUT_DIRECTORY}")
endif()

# Use generated rust files because cxx-build is unavailable yet. Steps to
# generate rust and C++ source files for frontend-client-cxx:
#
# 1. Uncomment `build = "build_cargo.rs"` and `cxx-build = "1.0"` in
#    rust/frontend-client-cxx/Cargo.toml.
# 2. cd rust && cargo build
# 3. git restore *-cxx/Cargo.toml
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
target_link_libraries(netsim-rust-lib INTERFACE netsim-cxx frontend-proto)

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
