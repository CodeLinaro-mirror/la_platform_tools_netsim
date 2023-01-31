/*
 * Copyright 2022 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#pragma once
// A synchronous Backend server for the Network Simulator.

#include <memory>
#include <string>
#include <utility>

#include "grpcpp/server.h"
#include "grpcpp/support/sync_stream.h"
#include "packet_streamer.pb.h"

namespace netsim {

class PacketStreamClient {
 public:
  PacketStreamClient(
      ::grpc::ServerReaderWriter< ::netsim::packet::StreamPacketsResponse,
                                  ::netsim::packet::StreamPacketsRequest>
          *stream);

  std::unique_ptr<std::string> Read() const;
  void Write(const std::string &) const;

 private:
  ::grpc::ServerReaderWriter< ::netsim::packet::StreamPacketsResponse,
                              ::netsim::packet::StreamPacketsRequest> *stream;
};

std::pair<std::unique_ptr<grpc::Server>, std::string> RunBackendServer();

}  // namespace netsim
