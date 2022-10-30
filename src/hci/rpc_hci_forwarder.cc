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

#include "hci/rpc_hci_forwarder.h"

#include <grpcpp/grpcpp.h>

#include <cstdint>
#include <fstream>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#include "emulated_bluetooth_vhci.grpc.pb.h"  // for VhciForwardingService
#include "emulated_bluetooth_vhci.pb.h"       // for HCIPacket
#include "hci/hci_debug.h"
#include "hci/rpc_transport.h"
#include "model/hci/hci_transport.h"
#include "util/log.h"

namespace netsim {
namespace hci {
namespace {

// Transports HCIPacket between grpc and chip emulator.
//
// Private implementation.
//
class RpcHciForwarderImpl : public RpcHciForwarder {
 public:
  ~RpcHciForwarderImpl() {}

  // Called by HCITransport (rootcanal)
  void SendEvent(const std::vector<uint8_t> &data) override {
    HCIPacket packet;
    packet.set_type(HCIPacket::PACKET_TYPE_EVENT);
    packet.set_packet(std::string(data.begin(), data.end()));
    auto event = HciEventToString(data);
    BtsLog("> %s %s", mSerial.c_str(), event.c_str());
    this->Write(packet);
  }

  // Called by HCITransport (rootcanal)
  void SendAcl(const std::vector<uint8_t> &data) override {
    HCIPacket packet;
    packet.set_type(HCIPacket::PACKET_TYPE_ACL);
    packet.set_packet(std::string(data.begin(), data.end()));
    BtsLog("< %s ACL", mSerial.c_str());
    this->Write(packet);
  }

  // Called by HCITransport (rootcanal)
  void SendSco(const std::vector<uint8_t> &data) override {
    HCIPacket packet;
    packet.set_type(HCIPacket::PACKET_TYPE_SCO);
    packet.set_packet(std::string(data.begin(), data.end()));
    BtsLog("> %s SCO", mSerial.c_str());
    this->Write(packet);
  }

  // Called by HCITransport (rootcanal)
  void SendIso(const std::vector<uint8_t> &data) override {
    HCIPacket packet;
    packet.set_type(HCIPacket::PACKET_TYPE_ISO);
    packet.set_packet(std::string(data.begin(), data.end()));
    BtsLog("> %s ISO", mSerial.c_str());
    this->Write(packet);
  }

  // Called by HCITransport (rootcanal)
  void RegisterCallbacks(rootcanal::PacketCallback commandCallback,
                         rootcanal::PacketCallback aclCallback,
                         rootcanal::PacketCallback scoCallback,
                         rootcanal::PacketCallback isoCallback,
                         rootcanal::CloseCallback closeCallback) override {
    std::cerr << "RegisterCallbacks ****" << std::endl;
    mCommandCallback = commandCallback;
    mAclCallback = aclCallback;
    mScoCallback = scoCallback;
    mIsoCallback = isoCallback;
    mCloseCallback = closeCallback;
  }

  // Called by HCITransport (rootcanal)
  void TimerTick() override {}

  void Close() override {
    const std::lock_guard<std::recursive_mutex> lock(mClosingMutex);
    if (!mClosed) {
      Finish(grpc::Status::CANCELLED);
    }
  }

  // Called by RpcTransport
  void Read(const HCIPacket *packet) override {
    auto data = std::make_shared<std::vector<uint8_t>>(packet->packet().begin(),
                                                       packet->packet().end());

    switch (packet->type()) {
      case HCIPacket::PACKET_TYPE_HCI_COMMAND: {
        auto cmd = HciCommandToString(data->at(0), data->at(1));
        BtsLog("< %s %s", mSerial.c_str(), cmd.c_str());
        mCommandCallback(data);
      } break;
      case HCIPacket::PACKET_TYPE_ACL:
        BtsLog("< %s ACL", mSerial.c_str());
        mAclCallback(data);
        break;
      case HCIPacket::PACKET_TYPE_SCO:
        BtsLog("< %s SCO", mSerial.c_str());
        mScoCallback(data);
        break;
      case HCIPacket::PACKET_TYPE_ISO:
        BtsLog("< %s ISO", mSerial.c_str());
        mIsoCallback(data);
        break;
      case HCIPacket::PACKET_TYPE_EVENT:
        BtsLog("Ignoring event packet.");
        break;
      default:
        BtsLog("Ignoring unknown packet.");
        break;
    }
  }

  // Called by RpcTransport
  void OnDone() override {
    BtsLog("OnDone %s", mSerial.c_str());
    const std::lock_guard<std::recursive_mutex> lock(mClosingMutex);
    mClosed = true;
    if (mCloseCallback) {
      mCloseCallback();
      mCloseCallback = nullptr;
    }
    // Decrease smart pointer references count
    mSelf.reset();
  }

  RpcHciForwarderImpl(std::string serial) : mSerial(serial) {}

  rootcanal::PacketCallback mAclCallback;
  std::shared_ptr<RpcHciForwarderImpl> mSelf;
  rootcanal::PacketCallback mCommandCallback;
  rootcanal::PacketCallback mScoCallback;
  rootcanal::PacketCallback mIsoCallback;
  rootcanal::CloseCallback mCloseCallback;
  std::recursive_mutex mClosingMutex;
  bool mClosed;

 private:
  std::string mSerial;
};

}  // namespace

// Factory constructor that returns the private implementation
//
std::shared_ptr<RpcHciForwarder> RpcHciForwarder::Create(std::string serial) {
  auto forwarder = std::make_shared<RpcHciForwarderImpl>(serial);
  // TODO - handle close callback so parent, not child releases handle
  forwarder->mSelf = forwarder;
  return forwarder;
}

}  // namespace hci
}  // namespace netsim
