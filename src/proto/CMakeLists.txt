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

cmake_minimum_required(VERSION 3.5)

protobuf_generate_grpc_cpp(
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}
  SOURCES ${CMAKE_CURRENT_LIST_DIR}/frontend.proto
          ${CMAKE_CURRENT_LIST_DIR}/internal.proto
          ${CMAKE_CURRENT_LIST_DIR}/model.proto
          ${CMAKE_CURRENT_LIST_DIR}/startup.proto
          ${CMAKE_CURRENT_LIST_DIR}/packet_streamer.proto
  OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}
  GENERATED GRPC_SRC)

android_add_library(TARGET frontend-grpc-lib LICENSE Apache-2.0 SRC ${GRPC_SRC}
                    DEPS grpc++ protobuf::libprotobuf)

target_include_directories(frontend-grpc-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..
                           PUBLIC ${CMAKE_CURRENT_BINARY_DIR})

protobuf_generate_grpc_cpp(
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}
  SOURCES ${CMAKE_CURRENT_LIST_DIR}/startup.proto
          ${CMAKE_CURRENT_LIST_DIR}/packet_streamer.proto
  OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}
  GENERATED PACKET_STREAMER_CLIENT_GRPC_SRC)

android_add_library(TARGET packet-streamer-client-grpc-lib LICENSE Apache-2.0 SRC ${PACKET_STREAMER_CLIENT_GRPC_SRC}
                    DEPS grpc++ protobuf::libprotobuf)

target_include_directories(packet-streamer-client-grpc-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..
                    PUBLIC ${CMAKE_CURRENT_BINARY_DIR})
