# Copyright 2022 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Netsim - a network simulator for discovery, ranging and communication

project(netsim)
cmake_minimum_required(VERSION 3.5)

set(CMAKE_BUILD_TYPE Debug)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED True)

set(BLUETOOTH_EMULATION True)

# Toolchain doesn't work so do this:
#
# $REPO/prebuilts/cmake/linux-x86/bin/cmake \
#  -G Ninja \
#  -DCMAKE_TOOLCHAIN_FILE=$REPO/external/qemu/android/build/cmake/toolchain-linux-x86_64.cmake \
#  ../CMakeLists.txt

set(EXTERNAL ${CMAKE_CURRENT_LIST_DIR}/../../external)
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


add_subdirectory(src)


add_executable(netsim src/netsim.cc)
target_compile_definitions(netsim PUBLIC NETSIM_ANDROID_EMULATOR)

target_link_libraries(netsim
  PRIVATE
  netsim-lib)

target_link_options(netsim PRIVATE -static-libstdc++)

# Testing

enable_testing()

include(GoogleTest)

add_executable(netsim-test
  src/controller/device_test.cc
  src/controller/scene_controller_test.cc
  src/hci/startup_test.cc
  src/util/os_utils_test.cc
  src/util/string_utils_test.cc
  src/util/ini_file_test.cc
  src/fe/frontend_server_test.cc)

target_link_libraries(netsim-test
  gtest
  gtest_main
  util-lib
  libbt-rootcanal
  netsim-lib
  frontend-lib
  controller-lib
  core-lib)

target_include_directories(
  netsim-test
  PRIVATE
  ${CMAKE_SOURCE_DIR}/src
  )

add_test(NAME netsim-test COMMAND netsim-test)
