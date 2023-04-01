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

android_add_library(
  TARGET frontend-client LICENSE Apache-2.0 SRC frontend_client.cc
                                                frontend_client.h
  DEPS grpc++ netsim-cli-proto-lib protobuf::libprotobuf util-lib)

target_include_directories(frontend-client PRIVATE ..)

android_add_library(
  TARGET frontend-lib
  LICENSE Apache-2.0
  SRC frontend_server.cc frontend_server.h server_response_writable.h
  DEPS controller-lib grpc++ netsim-cli-proto-lib netsim-cxx-generated
       # TODO: refactor so this isn't needed
       protobuf::libprotobuf util-lib)

target_include_directories(frontend-lib PRIVATE .. ../../rust/netsim-cxx
                                                ../../rust/netsim-cxx/cxx)
