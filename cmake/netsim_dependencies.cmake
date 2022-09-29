
set(BLUETOOTH_EMULATION True)

set(EXTERNAL ${CMAKE_CURRENT_LIST_DIR}/../../../external)
set(EXTERNAL_QEMU ${EXTERNAL}/qemu)

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

message("Building outside of emulator build..")
enable_testing()
include(android)
include(prebuilts)
prebuilt(Threads)
add_subdirectory(${EXTERNAL_QEMU}/android/third_party/abseil-cpp abseil-cpp)

add_subdirectory(${EXTERNAL_QEMU}/android/third_party/re2 re2)
set(_gRPC_RE2_INCLUDE_DIR "${EXTERNAL_QEMU}/android/third_party/re2")
set(_gRPC_RE2_LIBRARIES re2)

add_subdirectory(${EXTERNAL}/cares cares)
add_subdirectory(${EXTERNAL}/grpc/emulator grpc)

add_subdirectory(${EXTERNAL_QEMU}/android/third_party/boringssl boringssl)

add_subdirectory(${EXTERNAL_QEMU}/android/third_party/lz4 lz4)

add_subdirectory(${EXTERNAL_QEMU}/android/third_party/google-benchmark
                   google-benchmark)

add_subdirectory(${EXTERNAL_QEMU}/android/third_party/googletest/
                     gtest)

add_subdirectory(${EXTERNAL_QEMU}/android/bluetooth/rootcanal rootcanal)

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

