# Copyright 2023 The Android Open Source Project
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
  TARGET wifi-lib
  LICENSE Apache-2.0
  SRC wifi_facade.cc wifi_facade.h wifi_packet_hub.cc wifi_packet_hub.h
  DEPS controller-lib frontend-grpc-lib packet-streamer-server-lib
       protobuf::libprotobuf)
target_include_directories(wifi-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..)
target_compile_definitions(wifi-lib PUBLIC NETSIM_ANDROID_EMULATOR)
