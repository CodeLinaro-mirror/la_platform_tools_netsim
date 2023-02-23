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
#include <stdlib.h>

#include <memory>
#include <string>
#include <unordered_map>

#include "common.pb.h"
#include "controller/controller.h"
#include "google/protobuf/empty.pb.h"
#include "grpcpp/server_context.h"
#include "grpcpp/support/status.h"
#include "packet_hub/packet_hub.h"
#include "packet_streamer.grpc.pb.h"
#include "packet_streamer.pb.h"
#include "util/log.h"

namespace netsim {
namespace backend::grpc {
namespace {

using netsim::common::ChipKind;

using Stream =
    ::grpc::ServerReaderWriter<packet::PacketResponse, packet::PacketRequest>;

using netsim::startup::Chip;

std::unordered_map<uint32_t, Stream *> bt_facade_to_stream;

// Service handles the gRPC StreamPackets requests.

class ServiceImpl final : public packet::PacketStreamer::Service {
 public:
  ::grpc::Status StreamPackets(::grpc::ServerContext *context,
                               Stream *stream) override {
    // Now connected to a peer issuing a bi-directional streaming grpc
    auto peer = context->peer();
    BtsLog("backend_server new packet_stream for peer %s", peer.c_str());

    packet::PacketRequest request;

    // First packet must have initial_info describing the peer
    bool success = stream->Read(&request);
    if (!success || !request.has_initial_info()) {
      BtsLog("ServiceImpl no initial information or stream closed");
      return ::grpc::Status(::grpc::StatusCode::INVALID_ARGUMENT,
                            "Missing initial_info in first packet.");
    }

    auto device_name = request.initial_info().name();
    auto chip_kind = request.initial_info().chip().kind();
    // multiple chips of the same chip_kind for a device have a name
    auto chip_name = request.initial_info().chip().id();
    auto manufacturer = request.initial_info().chip().manufacturer();
    auto product_name = request.initial_info().chip().product_name();
    // Add a new chip to the device
    auto [device_id, chip_id, facade_id] = scene_controller::AddChip(
        peer, device_name, chip_kind, chip_name, manufacturer, product_name);

    BtsLog("backend_server: adding chip %d with facade %d to %s", chip_id,
           facade_id, device_name.c_str());
    // connect packet responses from chip facade to the peer
    bt_facade_to_stream[facade_id] = stream;
    this->ProcessRequests(stream, device_id, chip_kind, facade_id);

    // no longer able to send responses to peer
    bt_facade_to_stream.erase(facade_id);

    // Remove the chip from the device
    scene_controller::RemoveChip(device_id, chip_id);

    BtsLog("backend_server: removing chip %d from %s", chip_id,
           device_name.c_str());

    return ::grpc::Status::OK;
  }

  // Convert a protobuf bytes field into shared_ptr<<vec<uint8_t>>.
  //
  // Release ownership of the bytes field and convert it to a vector using move
  // iterators. No copy when called with a mutable reference.
  std::shared_ptr<std::vector<uint8_t>> ToSharedVec(std::string *bytes_field) {
    return std::make_shared<std::vector<uint8_t>>(
        std::make_move_iterator(bytes_field->begin()),
        std::make_move_iterator(bytes_field->end()));
  }

  // Process requests in a loop forwarding packets to the packet_hub and
  // returning when the channel is closed.
  void ProcessRequests(Stream *stream, uint32_t device_id,
                       common::ChipKind chip_kind, uint32_t facade_id) {
    packet::PacketRequest request;
    while (true) {
      if (!stream->Read(&request)) {
        BtsLog("backend_server: reading stopped for %d", facade_id);
        break;
      }
      // All kinds possible (bt, uwb, wifi), but each rpc only streames one.
      if (chip_kind == common::ChipKind::BLUETOOTH) {
        if (!request.has_hci_packet()) {
          BtsLog("backend_server: unknown packet type from %d", facade_id);
          continue;
        }
        auto packet_type = request.hci_packet().packet_type();
        auto packet =
            ToSharedVec(request.mutable_hci_packet()->mutable_packet());
        packet_hub::handle_bt_request(facade_id, packet_type, packet);
      } else {
        // TODO: add WiFi & UWB here
        BtsLog("backend_server: unknown chip kind");
      }
    }
  }
};
}  // namespace

// handle_bt_response is called by packet_hub to forward a response to
// the gRPC stream associated with facade_id.
//
// When writing, the packet is copied because it is a shared_ptr and grpc++
// doesn't know about smart pointers.
void handle_bt_response(uint32_t facade_id,
                        packet::HCIPacket_PacketType packet_type,
                        const std::shared_ptr<std::vector<uint8_t>> packet) {
  auto stream = bt_facade_to_stream[facade_id];
  if (stream) {
    // TODO: lock or caller here because gRPC does not allow overlapping writes.
    packet::PacketResponse response;
    response.mutable_hci_packet()->set_packet_type(packet_type);
    auto str_packet = std::string(packet->begin(), packet->end());
    // TODO: check if Swap is available since copied in line above
    response.mutable_hci_packet()->set_packet(str_packet);
    if (!stream->Write(response)) {
      BtsLog("backend_server: write failed %d", facade_id);
    }
  } else {
    BtsLog("backend_server: no stream for %d", facade_id);
  }
}

}  // namespace backend::grpc

std::unique_ptr<packet::PacketStreamer::Service> GetBackendService() {
  return std::make_unique<backend::grpc::ServiceImpl>();
}
}  // namespace netsim
