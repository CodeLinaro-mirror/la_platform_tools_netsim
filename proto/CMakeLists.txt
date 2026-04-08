# Copyright 2022 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

cmake_minimum_required(VERSION 3.5)

set(ROOTCANAL_PROTO_DIR ${AOSP_ROOT}/tools/rootcanal/proto)

# For netsimd (netsimd-proto-lib)
protobuf_generate_grpc_cpp(
  SOURCE_DIR ${ROOTCANAL_PROTO_DIR} SOURCES rootcanal/configuration.proto
  OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR} GENERATED ROOTCANAL_PROTO_SRC)

# For netsimd (netsimd-proto-lib)
protobuf_generate_grpc_cpp(
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}
  SOURCES netsim/common.proto netsim/config.proto netsim/hci_packet.proto
          netsim/model.proto netsim/startup.proto netsim/stats.proto
  INCLUDES ${ROOTCANAL_PROTO_DIR}
  OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}
  GENERATED NETSIM_PROTO_SRC)

android_add_library(
  TARGET netsimd-proto-lib LICENSE Apache-2.0 SRC ${NETSIM_PROTO_SRC}
                                                  ${ROOTCANAL_PROTO_SRC}
  DEPS grpc++ protobuf::libprotobuf)

target_include_directories(netsimd-proto-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..
                           PUBLIC ${CMAKE_CURRENT_BINARY_DIR})

# For netsim-cli (netsim-cli-proto-lib)
protobuf_generate_grpc_cpp(
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR} SOURCES netsim/frontend.proto
  INCLUDES ${ROOTCANAL_PROTO_DIR} OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}
  GENERATED NETSIM_FRONTEND_GRPC_SRC)

android_add_library(
  TARGET netsim-cli-proto-lib LICENSE Apache-2.0 SRC ${NETSIM_FRONTEND_GRPC_SRC}
  DEPS grpc++ netsimd-proto-lib protobuf::libprotobuf)

target_include_directories(
  netsim-cli-proto-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..
  PUBLIC ${CMAKE_CURRENT_BINARY_DIR})

# For network packet streamer client (packet-streamer-proto-lib)
protobuf_generate_grpc_cpp(
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR} SOURCES netsim/packet_streamer.proto
  OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR} INCLUDES ${ROOTCANAL_PROTO_DIR}
  GENERATED NETSIM_PACKET_STREAMER_GRPC_SRC)

android_add_library(
  TARGET packet-streamer-proto-lib LICENSE Apache-2.0
  SRC ${NETSIM_PACKET_STREAMER_GRPC_SRC} DEPS grpc++ netsimd-proto-lib
                                              protobuf::libprotobuf)

target_include_directories(
  packet-streamer-proto-lib PRIVATE ${PROTOBUF_INCLUDE_DIR} ..
  PUBLIC ${CMAKE_CURRENT_BINARY_DIR})

android_license(TARGET "packet-streamer-proto-lib" LIBNAME None SPDX None
                LICENSE None LOCAL None)
android_license(TARGET "netsim-cli-proto-lib" LIBNAME None SPDX None
                LICENSE None LOCAL None)
