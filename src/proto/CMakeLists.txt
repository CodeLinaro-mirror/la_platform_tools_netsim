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
  SOURCES
	${CMAKE_CURRENT_LIST_DIR}/frontend.proto
	${CMAKE_CURRENT_LIST_DIR}/internal.proto
	${CMAKE_CURRENT_LIST_DIR}/model.proto
	${CMAKE_CURRENT_LIST_DIR}/startup.proto
  OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}
  GENERATED GRPC_SRC)

add_library(frontend-grpc-lib STATIC
  ${GRPC_SRC})

target_link_libraries(frontend-grpc-lib PRIVATE
  grpc++
  protobuf::libprotobuf)

target_include_directories(
  frontend-grpc-lib
  PRIVATE
  ${PROTOBUF_INCLUDE_DIR}
  ${CMAKE_SOURCE_DIR}/src
  PUBLIC
  ${CMAKE_BINARY_DIR}/src/proto
  )
