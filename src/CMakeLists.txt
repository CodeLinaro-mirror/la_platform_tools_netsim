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

protobuf_generate_grpc_cpp(
  SOURCE_DIR ${EXTERNAL_QEMU}/android/android-grpc
  SOURCES ${EXTERNAL_QEMU}/android/android-grpc/emulated_bluetooth_packets.proto
          ${EXTERNAL_QEMU}/android/android-grpc/emulated_bluetooth_vhci.proto
  OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}
  GENERATED BLUETOOTH_CLIENT_GRPC_SRC)

android_add_library(
  TARGET vhci-lib LICENSE Apache-2.0 SRC ${BLUETOOTH_CLIENT_GRPC_SRC}
  DEPS grpc++ protobuf::libprotobuf)

target_include_directories(
  vhci-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ${CMAKE_CURRENT_SOURCE_DIR}
  PUBLIC ${CMAKE_CURRENT_BINARY_DIR})

add_subdirectory(util)
add_subdirectory(core)
add_subdirectory(proto)
add_subdirectory(controller)
add_subdirectory(fe)
add_subdirectory(backend)
add_subdirectory(hci)

add_library(netsim-lib INTERFACE)
target_link_libraries(
  netsim-lib
  INTERFACE core-lib
            frontend-grpc-lib
            frontend-lib
	    backend-lib
            hci-lib
            util-lib
            vhci-lib
            protobuf::libprotobuf)

android_license(TARGET "netsim-lib" LIBNAME None SPDX None LICENSE None LOCAL None)
