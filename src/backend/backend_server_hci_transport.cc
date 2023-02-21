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

#include <condition_variable>
#include <memory>
#include <queue>
#include <thread>
#include <vector>

#include "hci/hci_debug.h"
#include "model/hci/hci_transport.h"
#include "packet_streamer.grpc.pb.h"
#include "packet_streamer.pb.h"
#include "util/blocking_queue.h"
#include "util/log.h"

using netsim::packet::HCIPacket;

namespace netsim {
namespace hci {
namespace {

// Transports HCIPacket between grpc and chip emulator.
//
// All outgoing messages to a gRPC stream must be single threaded.
// All incoming messages to Rootcanal must be single threaded.
//
// Private implementation.
//
class BackendServerHciTransportImpl : public BackendServerHciTransport {
 public:
  BackendServerHciTransportImpl(std::string peer, Stream *stream)
      : mPeer(peer), mStream(stream), mReading(true) {}

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
    BtsLog("hci_transport: registered");
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
    mReading = false;
  }

  // Transport packets from grpc to rootcanal. Return when connection is closed.
  void Transport() override {
    packet::PacketRequest request;

    // non-blocking and serialized writes to gRPC on a thread
    mWriter = std::move(std::thread([&] { startWriter(); }));

    while (mReading) {
      if (!mStream->Read(&request)) {
        BtsLog("hci_transport: reading stopped peer %s", mPeer.c_str());
        break;
      }
      if (!request.has_hci_packet()) {
        BtsLog("hci_transport Unknown packet type");
        continue;
      }
      auto str = request.mutable_hci_packet()->mutable_packet();
      auto packet =
          std::make_shared<std::vector<uint8_t>>(str->begin(), str->end());
      auto packet_type = request.hci_packet().packet_type();

      if (packet_type == HCIPacket::COMMAND) {
        auto cmd = HciCommandToString(packet->at(0), packet->at(1));
        BtsLog("hci_transport: from %s %s", mPeer.c_str(), cmd.c_str());
      }
      auto packet_callback = PacketTypeCallback(packet_type);
      if (packet_callback != nullptr) {
        BtsLog("hci_transport: from start %s", mPeer.c_str());
        packet_callback(packet);
        BtsLog("hci_transport: from done %s", mPeer.c_str());
      } else {
        BtsLog("hci_transport: bad packet_callback...");
      }
    }
    // stream is closed on return
    BtsLog("hci_transport: Transport finished peer %s", mPeer.c_str());
    Done();
  }

 private:
  void Write(packet::HCIPacket::PacketType packet_type,
             const std::vector<uint8_t> &data) {
    packet::PacketResponse response;
    response.mutable_hci_packet()->set_packet_type(packet_type);
    auto packet = std::string(data.begin(), data.end());
    response.mutable_hci_packet()->set_packet(packet);
    auto event = HciEventToString(data);
    BtsLog("hci_transport: to %s %s", mPeer.c_str(), event.c_str());
    // send from chip to gRPC
    mQueue.Push(response);
  }

  // Called when grpc read or write fails
  void Done() {
    BtsLog("hci_transport done for peer %s", mPeer.c_str());
    mReading = false;
    mQueue.Stop();
    if (mCloseCallback) {
      mCloseCallback();
      mCloseCallback = nullptr;
    }
  }

  rootcanal::PacketCallback PacketTypeCallback(
      packet::HCIPacket_PacketType packet_type) {
    switch (packet_type) {
      case HCIPacket::COMMAND:
        assert(mCommandCallback);
        return mCommandCallback;
      case HCIPacket::ACL:
        return mAclCallback;
      case HCIPacket::SCO:
        return mScoCallback;
      case HCIPacket::ISO:
        return mIsoCallback;
      default:
        BtsLog("hci_transport Ignoring unknown packet.");
        return nullptr;
    }
  }

  // Writing through grpc stream is not thread safe, so all writes all queue
  // into this writer thread.
  void startWriter() {
    packet::PacketResponse outgoing;
    while (mQueue.WaitAndPop(outgoing)) {
      if (!mStream->Write(outgoing)) break;
    }
    Done();
  }

  Stream *mStream;
  rootcanal::PacketCallback mAclCallback;
  rootcanal::PacketCallback mCommandCallback;
  rootcanal::PacketCallback mScoCallback;
  rootcanal::PacketCallback mIsoCallback;
  rootcanal::CloseCallback mCloseCallback;
  bool mReading;
  std::string mPeer;
  util::BlockingQueue<packet::PacketResponse> mQueue;
  std::thread mWriter;
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
