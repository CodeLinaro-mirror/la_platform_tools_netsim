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

#include "hci/rpc_hal_transport.h"

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
#include "hci/bluetooth_chip_emulator.h"
#include "hci/rpc_hci_forwarder.h"
#include "util/ini_file.h"
#include "util/os_utils.h"

namespace netsim {
namespace {

using android::emulation::bluetooth::VhciForwardingService;

const char *kFilenameFormat = "pid_%d.ini";

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
      std::cerr << "RpcHalTransportImpl::connect - not found" << std::endl;
      return false;
    }
    auto deadline = std::chrono::system_clock::now() + kConnectionDeadline;
    if (!channel->WaitForConnected(deadline)) {
      std::cerr << "Unable to connect to " << serial << std::endl;
      return false;
    }

    std::cerr << "VhciForwardingClient::attach " << serial << std::endl;

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

  // Connect all Grpc endpoints.
  bool discover() override {
    std::cerr << "Connecting to all grpc endpoints\n";
    bool discovered = false;
    auto path =
        osutils::GetDiscoveryDirectory().append("avd").append("running");
    if (!std::filesystem::exists(path)) {
      std::cerr << "Unable to find discovery directory: " << path << std::endl;
      return false;
    }

    for (const auto &entry : std::filesystem::directory_iterator(path)) {
      int pid = 0;
      if (!entry.is_regular_file() ||
          std::sscanf(entry.path().filename().c_str(), kFilenameFormat, &pid) !=
              1) {
        continue;
      }
      IniFile iniFile(entry.path());
      if (!iniFile.Read()) {
        std::cerr << "Unable to read ini file: " << entry.path() << std::endl;
        continue;
      }
      auto grpc = iniFile.Get("grpc.port");
      if (!grpc.has_value()) {
        std::cerr << "Unable to read grpc.port: " << entry.path() << std::endl;
        continue;
      }
      auto port = grpc.value();
      std::cerr << "creating new RpcHal device grpc.port=" << port << std::endl;
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
  std::cerr << "Creating RpcHalTransport\n";
  return std::make_unique<RpcHalTransportImpl>();
}

}  // namespace netsim
