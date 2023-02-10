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

#include "backend/backend_server_hci_transport.h"

#include <grpcpp/grpcpp.h>

#include <memory>
#include <vector>

#include "model/hci/hci_transport.h"
#include "packet_streamer.grpc.pb.h"
#include "packet_streamer.pb.h"
#include "util/log.h"

using netsim::packet::HCIPacket;

namespace netsim {
namespace hci {
namespace {

// Transports HCIPacket between grpc and chip emulator.
//
// Private implementation.
//
class BackendServerHciTransportImpl : public BackendServerHciTransport {
 public:
  BackendServerHciTransportImpl(std::string peer, Stream *stream)
      : mPeer(peer), mStream(stream) {}
  ~BackendServerHciTransportImpl() {}

  // Called by HCITransport (rootcanal)
  void SendEvent(const std::vector<uint8_t> &data) override {
    this->Write(HCIPacket::EVENT, data);
  }

  // Called by HCITransport (rootcanal)
  void SendAcl(const std::vector<uint8_t> &data) override {
    this->Write(HCIPacket::ACL, data);
  }

  // Called by HCITransport (rootcanal)
  void SendSco(const std::vector<uint8_t> &data) override {
    this->Write(HCIPacket::SCO, data);
  }

  // Called by HCITransport (rootcanal)
  void SendIso(const std::vector<uint8_t> &data) override {
    this->Write(HCIPacket::ISO, data);
  }

  // Called by HCITransport (rootcanal)
  void RegisterCallbacks(rootcanal::PacketCallback commandCallback,
                         rootcanal::PacketCallback aclCallback,
                         rootcanal::PacketCallback scoCallback,
                         rootcanal::PacketCallback isoCallback,
                         rootcanal::CloseCallback closeCallback) override {
    BtsLog("hci_transport RegisterCallbacks");
    mCommandCallback = commandCallback;
    mAclCallback = aclCallback;
    mScoCallback = scoCallback;
    mIsoCallback = isoCallback;
    mCloseCallback = closeCallback;
  }

  // Called by HCITransport (rootcanal)
  void TimerTick() override {}

  // Close from Rootcanal
  void Close() override {
    BtsLog("hci_transport close from rootcanal");
    mDone = true;
  }

  // Transport packets from grpc to rootcanal. Return when connection is closed.
  void Transport() override {
    packet::PacketRequest request;
    while (true) {
      bool reader_success = mStream->Read(&request);
      if (!reader_success || mDone) {
        break;
      }
      if (!request.has_hci_packet()) {
        BtsLog("hci_transport Unknown packet type");
        continue;
      }
      auto str = request.mutable_hci_packet()->mutable_packet();
      auto packet =
          std::make_shared<std::vector<uint8_t>>(str->begin(), str->end());
      switch (request.hci_packet().packet_type()) {
        case HCIPacket::COMMAND: {
          mCommandCallback(std::move(packet));
        } break;
        case HCIPacket::ACL:
          mAclCallback(std::move(packet));
          break;
        case HCIPacket::SCO:
          mScoCallback(std::move(packet));
          break;
        case HCIPacket::ISO:
          mIsoCallback(std::move(packet));
          break;
        default:
          BtsLog("hci_transport Ignoring unknown packet.");
          break;
      }
    }
    // stream is closed on return
    Done();
  }

 private:
  void Write(packet::HCIPacket::PacketType packet_type,
             const std::vector<uint8_t> &data) {
    packet::PacketResponse response;
    response.mutable_hci_packet()->set_packet_type(packet_type);
    auto packet = std::string(data.begin(), data.end());
    response.mutable_hci_packet()->set_packet(packet);
    // send from chip to host
    if (!mStream->Write(response)) {
      Done();
    }
  }

  // Called when grpc read or write fails
  void Done() {
    BtsLog("hci_transport done for peer %s", mPeer.c_str());
    mDone = true;
    if (mCloseCallback) {
      mCloseCallback();
      mCloseCallback = nullptr;
    }
  }

  Stream *mStream;
  rootcanal::PacketCallback mAclCallback;
  rootcanal::PacketCallback mCommandCallback;
  rootcanal::PacketCallback mScoCallback;
  rootcanal::PacketCallback mIsoCallback;
  rootcanal::CloseCallback mCloseCallback;
  bool mDone;
  std::string mPeer;
};

}  // namespace

// Factory constructor that returns the private implementation
//
std::shared_ptr<BackendServerHciTransport> BackendServerHciTransport::Create(
    std::string peer, Stream *stream) {
  return std::make_shared<BackendServerHciTransportImpl>(peer, stream);
}

}  // namespace hci
}  // namespace netsim
