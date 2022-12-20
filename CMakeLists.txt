# Copyright 2022 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

# Netsim - a network simulator for discovery, ranging and communication

project(netsim)
cmake_minimum_required(VERSION 3.5)

if(NOT ANDROID_EMULATOR_BUILD)
  message(ERROR "Netsim should be built with emulator")
  set(CMAKE_BUILD_TYPE Debug)
  set(CMAKE_CXX_STANDARD 17)
  set(CMAKE_CXX_STANDARD_REQUIRED True)
  list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_LIST_DIR}/cmake")
  include(netsim_dependencies)
endif()

if(NOT Rust_COMPILER AND NOT OPTION_ENABLE_SYSTEM_RUST)
  message(ERROR " No Rust for netsim ${Rust_COMPILER}")
endif()

add_subdirectory(rust)

add_subdirectory(src)
android_add_executable(
  TARGET netsim LICENSE Apache-2.0 INSTALL . SRC src/netsim.cc
  DEPS grpc++ libbt-rootcanal netsim-lib protobuf::libprotobuf)

android_target_dependency(netsim linux TCMALLOC_OS_DEPENDENCIES)

target_compile_definitions(netsim PUBLIC NETSIM_ANDROID_EMULATOR)

android_add_executable(
  TARGET netsimd LICENSE Apache-2.0 INSTALL . SRC src/netsimd.cc
  DEPS grpc++ libbt-rootcanal netsim-lib netsim-rust-lib protobuf::libprotobuf)

android_target_dependency(netsimd linux TCMALLOC_OS_DEPENDENCIES)

target_compile_definitions(netsimd PUBLIC NETSIM_ANDROID_EMULATOR)

android_add_test(
  TARGET netsim-test LICENSE Apache-2.0
  SRC src/backend/packet_streamer_client_test.cc
      src/backend/startup_test.cc
      src/controller/device_test.cc
      src/controller/scene_controller_test.cc
      src/frontend/frontend_server_test.cc
      src/util/ini_file_test.cc
      src/util/os_utils_test.cc
      src/util/string_utils_test.cc
  DEPS android-emu-base-headers
       controller-lib
       core-lib
       frontend-lib
       grpc++
       gtest
       gtest_main
       libbt-rootcanal
       netsim-lib
       netsim-rust-lib
       packet-streamer-client-lib
       protobuf::libprotobuf
       util-lib
  TEST_PARAMS --gtest_filter=-OsUtilsTest.GetDiscoveryDir)

add_dependencies(netsim-test netsimd emulator) # For PacketStreamerClientTest.

target_include_directories(netsim-test PRIVATE src)
