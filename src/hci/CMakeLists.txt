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

add_library(hci-lib STATIC
  rpc_hal_transport.h
  rpc_hal_transport.cc
  rpc_hci_forwarder.h
  rpc_hci_forwarder.cc
  fd_startup.h
  fd_startup.cc
  hci_chip_emulator.h
  hci_packet.h
  hci_debug.h
  hci_debug.cc
  rpc_transport.h
  hci_chip_emulator.h
  hci_chip_emulator.cc
)

target_include_directories(
  hci-lib
  PRIVATE
  ${PROTOBUF_INCLUDE_DIR}
  ${CMAKE_SOURCE_DIR}/src
  )

target_link_libraries(hci-lib
  PRIVATE
  controller-lib
  libbt-rootcanal
  grpc++
  vhci-lib
  frontend-lib
)

