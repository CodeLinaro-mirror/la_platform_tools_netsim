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

android_add_library(
  TARGET backend-lib
  LICENSE Apache-2.0
  SRC rpc_transport.cc rpc_transport.h
  DEPS android-emu-base
       android-emu-base-headers
       controller-lib
       frontend-grpc-lib
       grpc++
       libbt-rootcanal
       libbt-rootcanal-qemu)
target_include_directories(backend-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..)

android_add_library(
  TARGET packet-streamer-server-lib
  LICENSE Apache-2.0
  SRC backend_server.cc backend_server.h backend_server_hci_transport.cc
      backend_server_hci_transport.h
  DEPS frontend-grpc-lib
       grpc++
       libbt-rootcanal
       netsim-cxx
       netsim-cxx-generated
       protobuf::libprotobuf
       util-lib)

target_include_directories(
  packet-streamer-server-lib PRIVATE .. ../../rust/netsim-cxx
                                     ../../rust/netsim-cxx/cxx)

android_add_library(
  TARGET packet-streamer-client-lib
  LICENSE Apache-2.0
  SRC packet_streamer_client.cc packet_streamer_client.h
  DEPS android-emu-base
       android-emu-base-headers
       android-emu-base-process
       grpc++
       packet-streamer-client-grpc-lib
       protobuf::libprotobuf
       util-lib)

target_include_directories(packet-streamer-client-lib
                           PUBLIC ${CMAKE_CURRENT_LIST_DIR}/..)
