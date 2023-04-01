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

add_subdirectory(util)
add_subdirectory(core)
add_subdirectory(proto)
add_subdirectory(controller)
add_subdirectory(frontend)
add_subdirectory(backend)
add_subdirectory(hci)
add_subdirectory(wifi)
add_subdirectory(packet_hub)

add_library(netsim-lib INTERFACE)
target_link_libraries(
  netsim-lib
  INTERFACE core-lib
            netsim-cli-proto-lib
            frontend-lib
            hci-lib
            netsimd-proto-lib
            packet-hub-lib
            packet-streamer-proto-lib
            packet-streamer-client-lib
            packet-streamer-server-lib
            protobuf::libprotobuf
            util-lib)

android_license(TARGET "netsim-lib" LIBNAME None SPDX None LICENSE None
                LOCAL None)
