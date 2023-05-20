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

#include "hci/bluetooth_facade.h"

#include <sys/types.h>

#include <cassert>
#include <chrono>
#include <cstdint>
#include <future>
#include <iostream>
#include <memory>
#include <unordered_map>
#include <utility>

#include "hci/hci_packet_transport.h"
#include "model/hci/hci_sniffer.h"
#include "model/setup/async_manager.h"
#include "model/setup/test_command_handler.h"
#include "model/setup/test_model.h"
#include "netsim-cxx/src/lib.rs.h"
#include "rust/cxx.h"
#include "util/filesystem.h"
#include "util/log.h"

#ifndef NETSIM_ANDROID_EMULATOR
#include "net/posix/posix_async_socket_server.h"
#endif

using netsim::model::State;

namespace netsim::hci::facade {

int8_t SimComputeRssi(int send_id, int recv_id, int8_t tx_power);
void IncrTx(uint32_t send_id, rootcanal::Phy::Type phy_type);
void IncrRx(uint32_t receive_id, rootcanal::Phy::Type phy_type);

using namespace std::literals;
using namespace rootcanal;

using rootcanal::PhyDevice;
using rootcanal::PhyLayer;

class SimPhyLayer : public PhyLayer {
  // for constructor inheritance
  using PhyLayer::PhyLayer;

  // Overrides ComputeRssi in PhyLayerFactory to provide
  // simulated RSSI information using actual spatial
  // device positions.
  int8_t ComputeRssi(PhyDevice::Identifier sender_id,
                     PhyDevice::Identifier receiver_id,
                     int8_t tx_power) override {
    return SimComputeRssi(sender_id, receiver_id, tx_power);
  }

  // Overrides Send in PhyLayerFactory to add Rx/Tx statistics.
  void Send(std::vector<uint8_t> const &packet, int8_t tx_power,
            PhyDevice::Identifier sender_id) override {
    IncrTx(sender_id, type);
    for (const auto &device : phy_devices_) {
      if (sender_id != device->id) {
        IncrRx(device->id, type);
        device->Receive(packet, type,
                        ComputeRssi(sender_id, device->id, tx_power));
      }
    }
  }
};

class SimTestModel : public rootcanal::TestModel {
  // for constructor inheritance
  using rootcanal::TestModel::TestModel;

  std::unique_ptr<rootcanal::PhyLayer> CreatePhyLayer(
      PhyLayer::Identifier id, rootcanal::Phy::Type type) override {
    return std::make_unique<SimPhyLayer>(id, type);
  }
};

size_t phy_low_energy_index_;
size_t phy_classic_index_;

bool mStarted = false;
std::shared_ptr<rootcanal::AsyncManager> mAsyncManager;
rootcanal::AsyncUserId gSocketUserId{};
std::unique_ptr<SimTestModel> gTestModel;
rootcanal::ControllerProperties controller_properties_;

#ifndef NETSIM_ANDROID_EMULATOR
// test port
std::unique_ptr<rootcanal::TestCommandHandler> gTestChannel;
std::unique_ptr<rootcanal::TestChannelTransport> gTestChannelTransport;
std::shared_ptr<AsyncDataChannelServer> gTestSocketServer;
bool gTestChannelOpen{false};
constexpr int kDefaultTestPort = 7500;
#endif

namespace {
bool ChangedState(model::State a, model::State b) {
  return (b != model::State::UNKNOWN && a != b);
}

#ifndef NETSIM_ANDROID_EMULATOR

using ::android::net::PosixAsyncSocketServer;

void SetUpTestChannel() {
  gTestSocketServer = std::make_shared<PosixAsyncSocketServer>(
      kDefaultTestPort, mAsyncManager.get());

  gTestChannel = std::make_unique<rootcanal::TestCommandHandler>(*gTestModel);

  gTestChannelTransport = std::make_unique<rootcanal::TestChannelTransport>();
  gTestChannelTransport->RegisterCommandHandler(
      [](const std::string &name, const std::vector<std::string> &args) {
        mAsyncManager->ExecAsync(
            gSocketUserId, std::chrono::milliseconds(0), [name, args]() {
              BtsLog("CommandHandle name:%s", name.c_str());
              std::string args_str = "";
              for (auto arg : args) args_str += " " + arg;
              BtsLog("arg:%s", args_str.c_str());
              if (name == "END_SIMULATION") {
                BtsLog("END_SIMULATION");
              } else {
                gTestChannel->HandleCommand(name, args);
              }
            });
      });

  bool transport_configured = gTestChannelTransport->SetUp(
      gTestSocketServer, [](std::shared_ptr<AsyncDataChannel> conn_fd,
                            AsyncDataChannelServer *server) {
        BtsLog("Test channel connection accepted.");
        server->StartListening();
        if (gTestChannelOpen) {
          BtsLog("Warning: Only one connection at a time is supported");
          rootcanal::TestChannelTransport::SendResponse(
              conn_fd, "The connection is broken");
          return false;
        }
        gTestChannelOpen = true;
        gTestChannel->RegisterSendResponse(
            [conn_fd](const std::string &response) {
              rootcanal::TestChannelTransport::SendResponse(conn_fd, response);
            });

        conn_fd->WatchForNonBlockingRead([](AsyncDataChannel *conn_fd) {
          gTestChannelTransport->OnCommandReady(
              conn_fd, []() { gTestChannelOpen = false; });
        });
        return false;
      });

  gTestChannel->AddDevice({"beacon", "be:ac:01:55:00:01", "1000"});
  gTestChannel->AddDeviceToPhy({"0", "1"});
  gTestChannel->AddDevice({"beacon", "be:ac:01:55:00:02", "1000"});
  gTestChannel->AddDeviceToPhy({"1", "1"});
  gTestChannel->SetTimerPeriod({"5"});
  gTestChannel->StartTimer({});

  if (!transport_configured) {
    BtsLog("Error: Test channel SetUp failed.");
    return;
  }

  BtsLog("Test channel SetUp() successful");
}
#endif

}  // namespace

// Initialize the rootcanal library.
void Start() {
  if (mStarted) return;

  // When emulators restore from a snapshot the PacketStreamer connection to
  // netsim is recreated with a new (uninitialized) Rootcanal device. However
  // the Android Bluetooth Stack does not re-initialize the controller. Our
  // solution is for Rootcanal to recognize that it is receiving HCI commands
  // before a HCI Reset. The flag below causes a hardware error event that
  // triggers the Reset from the Bluetooth Stack.

  controller_properties_.quirks.hardware_error_before_reset = true;

  mAsyncManager = std::make_shared<rootcanal::AsyncManager>();
  // Get a user ID for tasks scheduled within the test environment.
  gSocketUserId = mAsyncManager->GetNextUserId();

  gTestModel = std::make_unique<SimTestModel>(
      std::bind(&rootcanal::AsyncManager::GetNextUserId, mAsyncManager),
      std::bind(&rootcanal::AsyncManager::ExecAsync, mAsyncManager,
                std::placeholders::_1, std::placeholders::_2,
                std::placeholders::_3),
      std::bind(&rootcanal::AsyncManager::ExecAsyncPeriodically, mAsyncManager,
                std::placeholders::_1, std::placeholders::_2,
                std::placeholders::_3, std::placeholders::_4),
      std::bind(&rootcanal::AsyncManager::CancelAsyncTasksFromUser,
                mAsyncManager, std::placeholders::_1),
      std::bind(&rootcanal::AsyncManager::CancelAsyncTask, mAsyncManager,
                std::placeholders::_1),
      [](const std::string & /* server */, int /* port */,
         rootcanal::Phy::Type /* phy_type */) { return nullptr; });

  // NOTE: 0:BR_EDR, 1:LOW_ENERGY. The order is used by bluetooth CTS.
  phy_classic_index_ = gTestModel->AddPhy(rootcanal::Phy::Type::BR_EDR);
  phy_low_energy_index_ = gTestModel->AddPhy(rootcanal::Phy::Type::LOW_ENERGY);

  // TODO: Remove test channel.
#ifdef NETSIM_ANDROID_EMULATOR
  auto testCommands = rootcanal::TestCommandHandler(*gTestModel);
  testCommands.RegisterSendResponse([](const std::string &) {});
  testCommands.SetTimerPeriod({"5"});
  testCommands.StartTimer({});
#else
  SetUpTestChannel();
#endif
  mStarted = true;
};

// Resets the root canal library.
void Stop() {
  // TODO: Fix TestModel::Reset() in test_model.cc.
  // gTestModel->Reset();
  mStarted = false;
}

void PatchPhy(int device_id, bool isAddToPhy, bool isLowEnergy) {
  auto phy_index = (isLowEnergy) ? phy_low_energy_index_ : phy_classic_index_;
  if (isAddToPhy) {
    gTestModel->AddDeviceToPhy(device_id, phy_index);
  } else {
    gTestModel->RemoveDeviceFromPhy(device_id, phy_index);
  }
}

class ChipInfo {
 public:
  uint32_t simulation_device;
  std::shared_ptr<rootcanal::HciSniffer> sniffer;
  std::shared_ptr<model::Chip::Bluetooth> model;
  std::shared_ptr<HciPacketTransport> transport;
  int le_tx_count = 0;
  int classic_tx_count = 0;
  int le_rx_count = 0;
  int classic_rx_count = 0;

  ChipInfo(uint32_t simulation_device,
           std::shared_ptr<rootcanal::HciSniffer> sniffer,
           std::shared_ptr<model::Chip::Bluetooth> model,
           std::shared_ptr<HciPacketTransport> transport)
      : simulation_device(simulation_device),
        sniffer(sniffer),
        model(model),
        transport(transport) {}
};

std::unordered_map<uint32_t, std::shared_ptr<ChipInfo>> id_to_chip_info_;

model::Chip::Bluetooth Get(uint32_t id) {
  model::Chip::Bluetooth model;
  if (id_to_chip_info_.find(id) != id_to_chip_info_.end()) {
    model.CopyFrom(*id_to_chip_info_[id]->model.get());
    auto chip_info = id_to_chip_info_[id];
    model.mutable_classic()->set_tx_count(chip_info->classic_tx_count);
    model.mutable_classic()->set_rx_count(chip_info->classic_rx_count);
    model.mutable_low_energy()->set_tx_count(chip_info->le_tx_count);
    model.mutable_low_energy()->set_rx_count(chip_info->le_rx_count);
  }
  return model;
}

void Reset(uint32_t id) {
  if (id_to_chip_info_.find(id) != id_to_chip_info_.end()) {
    auto chip_info = id_to_chip_info_[id];
    chip_info->le_tx_count = 0;
    chip_info->le_rx_count = 0;
    chip_info->classic_tx_count = 0;
    chip_info->classic_rx_count = 0;
  }
  model::Chip::Bluetooth model;
  model.mutable_classic()->set_state(model::State::ON);
  model.mutable_low_energy()->set_state(model::State::ON);
  Patch(id, model);
}

void Patch(uint32_t id, const model::Chip::Bluetooth &request) {
  if (id_to_chip_info_.find(id) == id_to_chip_info_.end()) {
    BtsLog("Patch an unknown id %d", id);
    return;
  }
  auto model = id_to_chip_info_[id]->model;
  auto device_index = id_to_chip_info_[id]->simulation_device;
  // Low_energy radio state
  auto request_state = request.low_energy().state();
  auto *le = model->mutable_low_energy();
  if (ChangedState(le->state(), request_state)) {
    le->set_state(request_state);
    PatchPhy(device_index, request_state == model::State::ON, true);
  }
  // Classic radio state
  request_state = request.classic().state();
  auto *classic = model->mutable_classic();
  if (ChangedState(classic->state(), request_state)) {
    classic->set_state(request_state);
    PatchPhy(device_index, request_state == model::State::ON, false);
  }
}

void Remove(uint32_t id) {
  BtsLog("Removing HCI chip for %d", id);
  id_to_chip_info_.erase(id);
  gTestModel->RemoveDevice(id);
  // rootcanal will call HciPacketTransport::Close().
}

// Rename AddChip(model::Chip, device, transport)

uint32_t Add(uint32_t simulation_device) {
  auto transport = std::make_shared<HciPacketTransport>(mAsyncManager);
  // rewrap the transport to include a sniffer
  auto sniffer = std::static_pointer_cast<HciSniffer>(
      rootcanal::HciSniffer::Create(transport));
  auto hci_device =
      std::make_shared<rootcanal::HciDevice>(sniffer, controller_properties_);

  // Use the `AsyncManager` to ensure that the `AddHciConnection` method is
  // invoked atomically, preventing data races.
  std::promise<uint32_t> facade_id_promise;
  auto facade_id_future = facade_id_promise.get_future();
  mAsyncManager->ExecAsync(
      gSocketUserId, std::chrono::milliseconds(0),
      [hci_device, &facade_id_promise]() {
        facade_id_promise.set_value(gTestModel->AddHciConnection(hci_device));
      });
  auto facade_id = facade_id_future.get();

  HciPacketTransport::Add(facade_id, transport);
  BtsLog("Creating HCI facade %d for device %d", facade_id, simulation_device);

  auto model = std::make_shared<model::Chip::Bluetooth>();
  model->mutable_classic()->set_state(model::State::ON);
  model->mutable_low_energy()->set_state(model::State::ON);

  id_to_chip_info_.emplace(
      facade_id,
      std::make_shared<ChipInfo>(simulation_device, sniffer, model, transport));
  return facade_id;
}

void IncrTx(uint32_t id, rootcanal::Phy::Type phy_type) {
  if (id_to_chip_info_.find(id) != id_to_chip_info_.end()) {
    auto chip_info = id_to_chip_info_[id];
    if (phy_type == rootcanal::Phy::Type::LOW_ENERGY) {
      chip_info->le_tx_count++;
    } else {
      chip_info->classic_tx_count++;
    }
  }
}

void IncrRx(uint32_t id, rootcanal::Phy::Type phy_type) {
  if (id_to_chip_info_.find(id) != id_to_chip_info_.end()) {
    auto chip_info = id_to_chip_info_[id];
    if (phy_type == rootcanal::Phy::Type::LOW_ENERGY) {
      chip_info->le_rx_count++;
    } else {
      chip_info->classic_rx_count++;
    }
  }
}

// TODO: Make SimComputeRssi invoke netsim::device::GetDistanceRust with dev
// flag
int8_t SimComputeRssi(int send_id, int recv_id, int8_t tx_power) {
  if (id_to_chip_info_.find(send_id) == id_to_chip_info_.end() ||
      id_to_chip_info_.find(recv_id) == id_to_chip_info_.end()) {
#ifdef NETSIM_ANDROID_EMULATOR
    // NOTE: Ignore log messages in Cuttlefish for beacon devices created by
    // test channel.
    BtsLog("Missing chip_info");
#endif
    return tx_power;
  }
  auto a = id_to_chip_info_[send_id]->simulation_device;
  auto b = id_to_chip_info_[recv_id]->simulation_device;
  auto distance = scene_controller::GetDistance(a, b);
  return netsim::DistanceToRssi(tx_power, distance);
}

void PatchCxx(uint32_t id,
              const rust::Slice<::std::uint8_t const> proto_bytes) {
  model::Chip::Bluetooth bluetooth;
  bluetooth.ParseFromArray(proto_bytes.data(), proto_bytes.size());
  Patch(id, bluetooth);
}

rust::Vec<::std::uint8_t> GetCxx(uint32_t id) {
  auto bluetooth = Get(id);
  std::vector<uint8_t> proto_bytes(bluetooth.ByteSizeLong());
  bluetooth.SerializeToArray(proto_bytes.data(), proto_bytes.size());
  rust::Vec<uint8_t> proto_rust_bytes;
  std::copy(proto_bytes.begin(), proto_bytes.end(),
            std::back_inserter(proto_rust_bytes));
  return proto_rust_bytes;
}

}  // namespace netsim::hci::facade
