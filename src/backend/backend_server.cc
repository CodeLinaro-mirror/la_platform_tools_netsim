// Copyright 2022 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#include "backend/backend_server.h"

#include <google/protobuf/util/json_util.h>

#include <memory>
#include <string>

#include "backend_cxx_generated.h"
#include "google/protobuf/empty.pb.h"
#include "grpcpp/security/server_credentials.h"
#include "grpcpp/server.h"
#include "grpcpp/server_builder.h"
#include "grpcpp/server_context.h"
#include "grpcpp/support/status.h"
#include "packet_streamer.grpc.pb.h"
#include "packet_streamer.pb.h"
#include "util/log.h"

namespace netsim {
namespace {

class BackendServer final : public packet::PacketStreamer::Service {
 public:
  ::grpc::Status StreamPackets(
      ::grpc::ServerContext *context,
      ::grpc::ServerReaderWriter<::netsim::packet::StreamPacketsResponse,
                                 ::netsim::packet::StreamPacketsRequest>
          *stream) override {
    BtsLog("Streaming packets");
    // TODO: Call StreamPacketHandler().
    return ::grpc::Status::OK;
  }
};

BackendServer service;
}  // namespace

PacketStreamClient::PacketStreamClient(
    ::grpc::ServerReaderWriter<::netsim::packet::StreamPacketsResponse,
                               ::netsim::packet::StreamPacketsRequest> *stream)
    : stream(stream) {}

void PacketStreamClient::Write(const std::string &response) const {
  ::netsim::packet::StreamPacketsResponse response_proto;
  google::protobuf::util::JsonStringToMessage(response, &response_proto);
  stream->Write(response_proto);
}
std::unique_ptr<std::string> PacketStreamClient::Read() const {
  ::netsim::packet::StreamPacketsRequest request_proto;
  stream->Read(&request_proto);
  std::string request;
  google::protobuf::util::MessageToJsonString(request_proto, &request);
  return std::make_unique<std::string>(request);
}

std::pair<std::unique_ptr<grpc::Server>, std::string> RunBackendServer() {
  grpc::ServerBuilder builder;
  int selected_port;
  builder.AddListeningPort("0.0.0.0:0", grpc::InsecureServerCredentials(),
                           &selected_port);
  builder.RegisterService(&service);
  std::unique_ptr<grpc::Server> server(builder.BuildAndStart());

  BtsLog("Backend server listening on localhost: %s",
         std::to_string(selected_port).c_str());
  return std::make_pair(std::move(server), std::to_string(selected_port));
}

}  // namespace netsim
