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

/// Transport of virtual controller messages (hci, uci, netlink) from
/// desktop emulator (emu) to simulator.

#include "backend/rpc_hal_transport.h"

#include <chrono>
#include <filesystem>
#include <map>
#include <memory>
#include <thread>

#include "controller/device.h"
#include "controller/scene_controller.h"
#include "emulated_bluetooth_vhci.grpc.pb.h"  // for VhciForwardingService
#include "emulated_bluetooth_vhci.pb.h"       // for HCIPacket
#include "grpcpp/create_channel.h"
#include "grpcpp/security/credentials.h"
#include "hci/bluetooth_facade.h"
#include "hci/rpc_hci_forwarder.h"
#include "util/ini_file.h"
#include "util/log.h"
#include "util/os_utils.h"

namespace netsim {
namespace {

using android::emulation::bluetooth::VhciForwardingService;

#ifdef WIN32
const wchar_t *kFilenameFormat = L"pid_%d.ini";
#else
const char *kFilenameFormat = "pid_%d.ini";
#endif

const std::chrono::duration kConnectionDeadline = std::chrono::seconds(1);

// Use a private implementation class in order to avoid
// the public class needing to include VhciForwardingClient
// and a cascade of private implementation classes.
class RpcHalTransportImpl : public RpcHalTransport {
 public:
  // Disallow copy and assign
  RpcHalTransportImpl(const RpcHalTransportImpl &) = delete;
  RpcHalTransportImpl &operator=(const RpcHalTransportImpl &) = delete;

  RpcHalTransportImpl() {}

  // Connect a single RpcHal endpoint

  // TODO: don't connect twice (when /run/user files remain)

  bool connect(int port, std::string serial) override {
    auto server = "localhost:" + std::to_string(port);

    std::shared_ptr<grpc::Channel> channel =
        grpc::CreateChannel(server, grpc::InsecureChannelCredentials());
    if (channel == nullptr) {
      BtsLog("RpcHalTransportImpl::connect - not found");
      return false;
    }
    auto deadline = std::chrono::system_clock::now() + kConnectionDeadline;
    if (!channel->WaitForConnected(deadline)) {
      BtsLog("Unable to connect to %s", serial.c_str());
      return false;
    }

    BtsLog("VhciForwardingClient::attach %s", serial.c_str());

    std::unique_ptr<VhciForwardingService::Stub> stub(
        VhciForwardingService::NewStub(channel));

    // grpc context and stream are owned and deallocated by HCIForwarder
    auto context = std::make_unique<grpc::ClientContext>();
    auto stream = stub->attachVhci(context.get());

    // TODO: add other HAL forwarders here

    auto forwarder = hci::RpcHciForwarder::Create(serial);

    // Add a new HCI device for this RpcHciTransport

    std::shared_ptr<rootcanal::HciTransport> transport = forwarder;
    hci::BluetoothChipEmulator::Get().AddHciConnection(serial, transport);

    // Start the rpc transport
    forwarder->Start(serial, std::move(stream), std::move(context));

    return true;
  }

  static auto isPidFile(const std::filesystem::directory_entry &entry) -> bool {
    if (!entry.is_regular_file()) {
      return false;
    }

    int pid = 0;
#ifndef _WIN32
    return std::sscanf(entry.path().filename().c_str(), kFilenameFormat,
                       &pid) != 1;
#else
    return std::swscanf(entry.path().filename().c_str(), kFilenameFormat,
                        &pid) != 1;
#endif
  }

  // Connect all Grpc endpoints.
  bool discover() override {
    BtsLog("Connecting to all grpc endpoints");
    bool discovered = false;
    auto path =
        osutils::GetDiscoveryDirectory().append("avd").append("running");
    if (!std::filesystem::exists(path)) {
      BtsLog("Unable to find discovery directory: %s", path.c_str());
      return false;
    }

    for (const auto &entry : std::filesystem::directory_iterator(path)) {
      if (!isPidFile(entry)) {
        continue;
      }
      IniFile iniFile(entry.path());
      if (!iniFile.Read()) {
        BtsLog("Unable to read ini file: %s", entry.path().c_str());
        continue;
      }
      auto grpc = iniFile.Get("grpc.port");
      if (!grpc.has_value()) {
        BtsLog("Unable to read grpc.port: %s", entry.path().c_str());
        continue;
      }
      auto port = grpc.value();
      BtsLog("Creating new RpcHal device grpc.port:%s", port.c_str());
      if (this->connect(std::stoi(port), "emulator-" + grpc.value())) {
        auto device =
            netsim::controller::CreateDevice("emulator-" + grpc.value());
        netsim::controller::SceneController::Singleton().Add(device);
        discovered = true;
      }
    }
    return discovered;
  }
};

}  // namespace

RpcHalTransport::~RpcHalTransport(){};

// Factory constructor for RpcHalTransport that returns the private
// implementation;

// TODO: singleton pattern

std::unique_ptr<RpcHalTransport> RpcHalTransport::Create() {
  BtsLog("Creating RpcHalTransport");
  return std::make_unique<RpcHalTransportImpl>();
}

}  // namespace netsim
