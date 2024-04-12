set(BLUETOOTH_EMULATION True)
get_filename_component(AOSP "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
set(EXTERNAL ${AOSP}/external)
set(EXTERNAL_QEMU ${EXTERNAL}/qemu)
set(ANDROID_QEMU2_TOP_DIR ${EXTERNAL_QEMU})

if(NOT Python_EXECUTABLE)
  find_package(Python3 COMPONENTS Interpreter)
  if(NOT Python3_FOUND)
    message(FATAL_ERROR "A python interpreter is required. ")
  endif()
  set(Python_EXECUTABLE ${Python3_EXECUTABLE})
endif()

message(STATUS "Using Python: ${Python_EXECUTABLE}")
if(NOT DEFINED ANDROID_TARGET_TAG)
  message(
    WARNING
      "You should invoke the cmake generator with a proper toolchain from ${EXTERNAL_QEMU}/android/build/cmake, "
      "Trying to infer toolchain, this might not work.")
  list(APPEND CMAKE_MODULE_PATH "${EXTERNAL_QEMU}/android/build/cmake/")
  include(toolchain)
  _get_host_tag(TAG)
  toolchain_configure_tags(${TAG})
endif()

include(android)
include(prebuilts)

# Append the given flags to the existing CMAKE_C_FLAGS. Be careful as these
# flags are global and used for every target! Note this will not do anything
# under vs for now
function(add_c_flag FLGS)
  foreach(FLAG ${FLGS})
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${FLAG}" PARENT_SCOPE)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${FLAG}" PARENT_SCOPE)
  endforeach()
endfunction()

function(add_cxx_flag FLGS)
  foreach(FLAG ${FLGS})
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${FLAG}" PARENT_SCOPE)
  endforeach()
endfunction()

if(WINDOWS_MSVC_X86_64)
  add_cxx_flag("-std:c++17")
else()
  add_cxx_flag("-std=c++17")
endif()
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
  add_definitions("-DANDROID_DEBUG")
  if(NOT WINDOWS_MSVC_X86_64)
    add_c_flag("-O0 -g3")
  else()
    add_c_flag("-Zi -Od")
  endif()

  if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang" AND NOT CROSSCOMPILE)
    if(NOT OPTION_ASAN AND OPTION_ASAN_IN_DEBUG)
      set(OPTION_ASAN address)
    endif()

    if(OPTION_ASAN STREQUAL "thread" AND OPTION_COVERAGE_IN_DEBUG)
      message(FATAL_ERROR "You cannot run tsan with code coverage enabled.")
    endif()
    if(NOT WINDOWS_MSVC_X86_64 AND OPTION_COVERAGE_IN_DEBUG)
      message("Enabling code coverage")
      # Build an instrumented version of the code  that generates coverage
      # mapping to enable code coverage analysis
      set(ANDROID_CODE_COVERAGE TRUE)
      add_c_flag("-fcoverage-mapping")
      add_c_flag("-fprofile-instr-generate")
      add_c_flag("-fprofile-arcs")
      add_c_flag("-ftest-coverage")
      add_c_flag("--coverage")
    endif()
  endif()
else()
  set(CMAKE_INSTALL_DO_STRIP TRUE)
  add_definitions("-DNDEBUG=1")
  if(WINDOWS_MSVC_X86_64)
    # clang-cl takes msvc based parameters, so -O3 is a nop
    add_c_flag("-O2")
  else()
    add_c_flag("-O3 -g3")
  endif()
endif()

# Target specific configurations that we do not want to do in the
# toolchain.cmake Toolchain variables seem to be overwritten pending your cmake
# version.
if(LINUX_X86_64)
  add_c_flag("-Werror")
  add_c_flag("-Wno-deprecated-declarations") # Protobuf generates deprecation
                                             # warnings for deprecated enums
  # And the asm type if we are compiling with yasm
  set(ANDROID_NASM_TYPE elf64)
  # This should make sure we have sufficient information left to properly print
  # std::string etc. see b/156534499 for details.
  add_c_flag("-fno-limit-debug-info")
elseif(LINUX_AARCH64)
  set(ANDROID_NASM_TYPE elf64)
  add_c_flag("-fpermissive")
elseif(WINDOWS_MSVC_X86_64)
  # And the asm type if we are compiling with yasm
  set(ANDROID_NASM_TYPE win64)
  set(CMAKE_SHARED_LIBRARY_PREFIX "lib")
elseif(DARWIN_X86_64 OR DARWIN_AARCH64)
  # And the asm type if we are compiling with yasm
  set(ANDROID_NASM_TYPE macho64)
  # Always consider the source to be darwin.
  add_definitions(-D_DARWIN_C_SOURCE=1)
  add_c_flag("-Wno-everything")
else()
  message(FATAL_ERROR "Unknown target!")
endif()

prebuilt(Threads)

# We need the auto generated header for some components, so let's set the
# ANDROID_HW_CONFIG_H variable to point to the generated header. Those that need
# it can add it to their sources list, and it will be there.
set(HW_PROPERTIES_INI
    ${EXTERNAL_QEMU}/android/emu/avd/src/android/avd/hardware-properties.ini)
android_generate_hw_config()

if(DARWIN_AARCH64 AND NOT Rust_COMPILER)
  message(
    STATUS
      "On Apple sillicon attempting to use platform toolchain if available.")
  list(APPEND CMAKE_MODULE_PATH
       "${EXTERNAL_QEMU}/android/build/cmake/corrosion/cmake/")
  find_package(Rust REQUIRED)
  if(TARGET Rust::Rustc)
    set(OPTION_ENABLE_SYSTEM_RUST TRUE)
  else()
    message(STATUS "Unable to derive local toolchain")
    message(
      FATAL_ERROR
        "If you are a developer you can install rust with `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`"
    )
  endif()
endif()

if(WINDOWS_MSVC_X86_64)
  # Set of msvc compat layer libraries.
  add_subdirectory(${EXTERNAL_QEMU}/android/third_party/mman-win32 mman-win32)
  add_subdirectory(${EXTERNAL_QEMU}/android/third_party/regex-win32 regex-win32)
  add_subdirectory(${EXTERNAL_QEMU}/android/third_party/dirent-win32
                   dirent-win32)
endif()

if(Rust_COMPILER OR OPTION_ENABLE_SYSTEM_RUST)
  if(OPTION_ENABLE_SYSTEM_RUST)
    message(STATUS "Attempting to use the system rust compiler")
    use_system_rust_toolchain()
  endif()

  enable_vendorized_crates("${EXTERNAL_QEMU}/android/third_party/rust/crates")
  add_subdirectory(${EXTERNAL_QEMU}/android/build/cmake/corrosion corrosion)
  ensure_rust_version_is_compliant()
endif()

set(_gRPC_RE2_INCLUDE_DIR "${EXTERNAL_QEMU}/android/third_party/re2")
set(_gRPC_RE2_LIBRARIES re2)
set(NETSIM_EXT TRUE)

# Let's bin place everything in the root, with the shared libs in the right
# place
set(DBG_INFO ${CMAKE_BINARY_DIR}/build/debug_info)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib64)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/archives)
set(CMAKE_PDB_OUTPUT_DIRECTORY ${DBG_INFO})
# Feeling courageous? Set this to $ANDROID_SDK_ROOT
if(DARWIN_X86_64 OR DARWIN_AARCH64)
  set(CMAKE_INSTALL_PREFIX ${CMAKE_BINARY_DIR}/distribution/emulator)
  set(CMAKE_INSTALL_CODESIGN ${CMAKE_BINARY_DIR}/distribution/_codesign)
else()
  set(CMAKE_INSTALL_PREFIX ${CMAKE_BINARY_DIR}/distribution/emulator)
endif()

# First make the protobuf and dependencies available to gRPC
add_subdirectory(${EXTERNAL}/qemu/android/third_party/protobuf protobuf)

add_subdirectory(${AOSP}/hardware/google/aemu/base aemu-base)
add_subdirectory(${AOSP}/hardware/google/aemu/host-common host-common)
add_subdirectory(${AOSP}/packages/modules/Bluetooth/tools/rootcanal rootcanal)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/abseil-cpp abseil-cpp)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/boringssl boringssl)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/google-benchmark
                 google-benchmark)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/hostapd hostapd)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/libslirp libslirp)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/googletest/ gtest)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/lz4 lz4)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/re2 re2)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/libselinux libselinux)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/libsparse libsparse)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/ext4_utils ext4_utils)
add_subdirectory(${EXTERNAL}/cares cares)
add_subdirectory(${EXTERNAL}/glib/glib glib2)
add_subdirectory(${EXTERNAL}/grpc/emulator grpc)
add_subdirectory(${EXTERNAL}/qemu/android/android-emu-base android-emu-base)
add_subdirectory(${EXTERNAL}/qemu/android/android-net/android android-emu-net)
add_subdirectory(${EXTERNAL}/qemu/android-qemu2-glue/netsim
                 android-wifi-service)
add_subdirectory(${EXTERNAL}/qemu/android/emu/base emu-base)
add_subdirectory(${EXTERNAL}/qemu/android/emu/utils android-emu-utils)
add_subdirectory(${EXTERNAL}/qemu/android/emu/files android-emu-files)
add_subdirectory(${EXTERNAL}/qemu/android/emu/agents android-emu-agents)
add_subdirectory(${EXTERNAL}/qemu/android/emu/proxy android-emu-proxy)
add_subdirectory(${EXTERNAL}/webrtc/third_party/jsoncpp jsoncpp)

# Short term fix for missing glib2 dll for Windows build
if(WINDOWS_MSVC_X86_64)
  install(TARGETS glib2_${ANDROID_TARGET_TAG} RUNTIME DESTINATION .
          LIBRARY DESTINATION .)
endif()

if(NOT TARGET gfxstream-snapshot.headers)
  # Fake dependency to satisfy linker
  add_library(gfxstream-snapshot.headers INTERFACE)
endif()

if(CMAKE_BUILD_TYPE MATCHES DEBUG)
  # This will help you find issues.
  set(CMAKE_C_FLAGS "-fsanitize=address -fno-omit-frame-pointer -g3 -O0")
  set(CMAKE_EXE_LINKER_FLAGS "-fsanitize=address")
endif()

if(LINUX_X86_64)
  # Our linux headers are from 2013, and do not define newer socket options.
  # (b/156635589)
  target_compile_options(grpc PRIVATE -DSO_REUSEPORT=15)
  target_compile_options(grpc_unsecure PRIVATE -DSO_REUSEPORT=15)
endif()

# Testing
enable_testing()
include(GoogleTest)

# Stripping flags for non-debug builds
if(NOT CMAKE_BUILD_TYPE STREQUAL "Debug")
  set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -s")
endif()
