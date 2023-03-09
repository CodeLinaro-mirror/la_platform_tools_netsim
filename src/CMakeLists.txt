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

add_subdirectory(proto)

android_add_library(
  TARGET util-lib
  LICENSE Apache-2.0
  SRC util/filesystem.h
      util/ini_file.cc
      util/ini_file.h
      util/log.cc
      util/log.h
      util/os_utils.cc
      util/os_utils.h
      util/string_utils.cc
      util/string_utils.h)
target_include_directories(util-lib PRIVATE .)

android_add_library(
  TARGET frontend-client LICENSE Apache-2.0 SRC frontend/frontend_client.cc
                                                frontend/frontend_client.h
  DEPS grpc++ netsim-cli-proto-lib protobuf::libprotobuf util-lib)
target_include_directories(frontend-client PRIVATE .)

android_add_library(
  TARGET packet-streamer-client-lib
  LICENSE Apache-2.0
  SRC backend/packet_streamer_client.cc backend/packet_streamer_client.h
  DEPS android-emu-base
       android-emu-base-headers
       android-emu-base-process
       grpc++
       packet-streamer-proto-lib
       protobuf::libprotobuf
       util-lib)
target_include_directories(packet-streamer-client-lib
                           PUBLIC ${CMAKE_CURRENT_LIST_DIR})

android_add_library(
  TARGET netsimd-lib
  LICENSE Apache-2.0
  SRC backend/backend_packet_hub.h
      backend/grpc_server.cc
      backend/grpc_server.h
      controller/chip.cc
      controller/chip.h
      controller/controller.cc
      controller/controller.h
      controller/device.cc
      controller/device.h
      controller/device_notify_manager.cc
      controller/device_notify_manager.h
      controller/scene_controller.cc
      controller/scene_controller.h
      core/server.cc
      core/server.h
      frontend/
      frontend/frontend_server.cc
      frontend/frontend_server.h
      frontend/server_response_writable.h
      hci/bluetooth_facade.cc
      hci/bluetooth_facade.h
      hci/hci_debug.cc
      hci/hci_debug.h
      hci/hci_packet_transport.cc
      hci/hci_packet_transport.h
      packet_hub/packet_hub.cc
      packet_hub/packet_hub.h
      wifi/wifi_facade.cc
      wifi/wifi_facade.h
      wifi/wifi_packet_hub.h
  DEPS android-emu-base
       android-emu-base-headers
       grpc++
       libbt-rootcanal
       libbt-rootcanal-qemu
       netsim-cxx
       netsim-cxx-generated
       netsimd-proto-lib
       protobuf::libprotobuf
       util-lib)
target_include_directories(
  netsimd-lib PRIVATE . ${PROTOBUF_INCLUDE_DIR} ../rust/netsim-cxx
                      ../rust/netsim-cxx/cxx)
target_compile_definitions(netsimd-lib PUBLIC NETSIM_ANDROID_EMULATOR)
