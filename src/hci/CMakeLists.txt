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
  TARGET hci-lib
  LICENSE Apache-2.0
  SRC fd_startup.cc
      fd_startup.h
      bluetooth_chip_emulator.cc
      bluetooth_chip_emulator.h
      hci_debug.cc
      hci_debug.h
      hci_packet.h
      rpc_hal_transport.cc
      rpc_hal_transport.h
      rpc_hci_forwarder.cc
      rpc_hci_forwarder.h
      rpc_transport.h
  DEPS android-emu-base
       android-emu-base-headers
       controller-lib
       frontend-grpc-lib
       frontend-lib
       grpc++
       libbt-rootcanal
       libbt-rootcanal-qemu
       vhci-lib)
target_include_directories(hci-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..)
