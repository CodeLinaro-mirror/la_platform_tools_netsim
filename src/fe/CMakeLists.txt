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

add_library(frontend-lib STATIC
  cli.h
  cli.cc
  frontend_server.h
  frontend_server.cc)

target_link_libraries(frontend-lib
  controller-lib
  frontend-grpc-lib
# TODO: refactor so this isn't needed
  libbt-rootcanal
  util-lib
  grpc++
  protobuf::libprotobuf)

target_include_directories(
  frontend-lib
  PRIVATE
  ${CMAKE_SOURCE_DIR}/src
  )
