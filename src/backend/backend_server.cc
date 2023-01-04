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

#include "backend/backend_server_hci_transport.h"
#include "google/protobuf/empty.pb.h"
#include "grpcpp/security/server_credentials.h"
#include "grpcpp/server.h"
#include "grpcpp/server_builder.h"
#include "grpcpp/server_context.h"
#include "grpcpp/support/status.h"
#include "hci/bluetooth_facade.h"
#include "packet_streamer.grpc.pb.h"
#include "packet_streamer.pb.h"
#include "util/log.h"

namespace netsim {
namespace {

using Stream = ::grpc::ServerReaderWriter<packet::PacketResponse,
                                          packet::PacketRequest>;

// Service handles grpc requests
//
class ServiceImpl final : public packet::PacketStreamer::Service {
 public:
  ServiceImpl(){};

  ::grpc::Status StreamPackets(::grpc::ServerContext *context,
                               Stream *stream) override {
    // Now connected to a peer issuing a bi-directional streaming grpc
    auto peer = context->peer();
    BtsLog("Streaming packets");

    packet::PacketRequest request;

    // First packet must have initial_info describing the peer
    bool success = stream->Read(&request);
    if (success || !request.has_initial_info()) {
      BtsLog("ServiceImpl no initial information");
      return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT,
                          "Missing initial_info in first packet.");
    }

    auto serial = request.initial_info().serial();
    auto kind = request.initial_info().chip().kind();

    auto bs_hci_transport =
        hci::BackendServerHciTransport::Create(peer, stream);
    std::shared_ptr<rootcanal::HciTransport> transport = bs_hci_transport;

    // Add a new HCI device for this RpcHciTransport
    hci::BluetoothChipEmulator::Get().AddHciConnection(serial, transport);
    bs_hci_transport->Transport();
    return ::grpc::Status::OK;
  }
};

}  // namespace

// Runs the BackendServer.
//
std::pair<std::unique_ptr<grpc::Server>, std::string> RunBackendServer() {
  // process lifetime for service
  static auto service = ServiceImpl();

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
