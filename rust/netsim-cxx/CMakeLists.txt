message(STATUS "Adding rust ${CMAKE_CURRENT_LIST_DIR}/Cargo.toml")

# This will automatically register all the tests as well.
corrosion_import_crate(MANIFEST_PATH Cargo.toml FLAGS --offline --verbose)

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

android_add_library(
  TARGET netsim-cxx-generated
  LICENSE Apache-2.0
  SRC ${CMAKE_CURRENT_LIST_DIR}/cxx/netsim_cxx_generated.cc
      ${CMAKE_CURRENT_LIST_DIR}/cxx/netsim_cxx_generated.h
  DEPS frontend-grpc-lib grpc++ packet-streamer-server-lib)

target_include_directories(
  netsim-cxx-generated PRIVATE ../../src # For backend/backend_server.h
                               . # For cxx/cxx.h
)
