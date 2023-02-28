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

#include <cassert>
#include <chrono>
#include <iostream>
#include <memory>
#include <unordered_map>
#include <utility>

#include "controller/chip.h"
#include "controller/scene_controller.h"
#include "model/devices/link_layer_socket_device.h"
#include "model/hci/hci_sniffer.h"
#include "model/hci/hci_socket_transport.h"
#include "model/setup/async_manager.h"
#include "model/setup/test_command_handler.h"
#include "model/setup/test_model.h"
#include "netsim_cxx_generated.h"
#include "packet/raw_builder.h"  // for RawBuilder
#include "util/filesystem.h"
#include "util/log.h"

using netsim::model::State;

namespace netsim {
namespace hci {
namespace {

using namespace std::literals;
using namespace rootcanal;

/// Transport wrapper for transports that run on an auxiliary thread.
/// Helps reschedule packet handling to the AsyncManager event thread
/// to ensure synchronization with other RootCanal events.
class SyncTransport : public HciTransport {
 public:
  SyncTransport(std::shared_ptr<HciTransport> transport,
                AsyncManager &async_manager)
      : mTransport(std::move(transport)), mAsyncManager(async_manager) {}
  ~SyncTransport() = default;

  void RegisterCallbacks(PacketCallback cmd_callback,
                         PacketCallback acl_callback,
                         PacketCallback sco_callback,
                         PacketCallback iso_callback,
                         CloseCallback close_callback) override {
    mTransport->RegisterCallbacks(
        [this, cmd_callback = std::move(cmd_callback)](
            const std::shared_ptr<std::vector<uint8_t>> cmd) {
          mAsyncManager.Synchronize(
              [cmd_callback, cmd = std::move(cmd)]() { cmd_callback(cmd); });
        },
        [this, acl_callback = std::move(acl_callback)](
            const std::shared_ptr<std::vector<uint8_t>> acl) {
          mAsyncManager.Synchronize(
              [acl_callback, acl = std::move(acl)]() { acl_callback(acl); });
        },
        [this, sco_callback = std::move(sco_callback)](
            const std::shared_ptr<std::vector<uint8_t>> sco) {
          mAsyncManager.Synchronize(
              [sco_callback, sco = std::move(sco)]() { sco_callback(sco); });
        },
        [this, iso_callback = std::move(iso_callback)](
            const std::shared_ptr<std::vector<uint8_t>> iso) {
          mAsyncManager.Synchronize(
              [iso_callback, iso = std::move(iso)]() { iso_callback(iso); });
        },
        close_callback);
  }

  void SendEvent(const std::vector<uint8_t> &packet) override {
    mTransport->SendEvent(packet);
  }
  void SendAcl(const std::vector<uint8_t> &packet) override {
    mTransport->SendAcl(packet);
  }
  void SendSco(const std::vector<uint8_t> &packet) override {
    mTransport->SendSco(packet);
  }
  void SendIso(const std::vector<uint8_t> &packet) override {
    mTransport->SendIso(packet);
  }

  void Tick() override { mTransport->Tick(); }
  void Close() override { mTransport->Close(); }

 private:
  std::shared_ptr<HciTransport> mTransport;
  AsyncManager &mAsyncManager;
};

int8_t ComputeRssi(int send_id, int recv_id, int8_t tx_power);
void IncrTx(uint32_t send_id, rootcanal::Phy::Type phy_type);
void IncrRx(uint32_t receive_id, rootcanal::Phy::Type phy_type);

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
    return netsim::hci::ComputeRssi(sender_id, receiver_id, tx_power);
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

class BluetoothChip;

// Private implementation class for Bluetooth BluetoothChipEmulator, a facade
// for Rootcanal library.

class BluetoothChipEmulatorImpl : public BluetoothChipEmulator {
 public:
  BluetoothChipEmulatorImpl() {}
  ~BluetoothChipEmulatorImpl() {}

  BluetoothChipEmulatorImpl(const BluetoothChipEmulatorImpl &) = delete;

  // Initialize the rootcanal library.
  void Start(std::string rootcanal_default_commands_file,
             std::string rootcanal_controller_properties_file) override {
    if (mStarted) return;
    controller_properties_ = rootcanal_controller_properties_file;

    // NOTE: 0:BR_EDR, 1:LOW_ENERGY. The order is used by bluetooth CTS.
    phy_classic_index_ = mTestModel.AddPhy(rootcanal::Phy::Type::BR_EDR);
    phy_low_energy_index_ = mTestModel.AddPhy(rootcanal::Phy::Type::LOW_ENERGY);

    // TODO: remove testCommands
    auto testCommands = rootcanal::TestCommandHandler(mTestModel);
    testCommands.RegisterSendResponse([](const std::string &) {});
    testCommands.SetTimerPeriod({"5"});
    testCommands.StartTimer({});
    testCommands.FromFile(rootcanal_default_commands_file);

    mStarted = true;
  };

  void AddHciConnection(
      const std::string &name,
      std::shared_ptr<rootcanal::HciTransport> transport) override;

  std::shared_ptr<BluetoothChip> Get(int device_index);
  void Remove(int device_index);

  // Resets the root canal library.
  // TODO: rename to Reset()
  void Close() override {
    mTestModel.Reset();
    mStarted = false;
  }

  int8_t ComputeRssi(int send_id, int recv_id, int8_t tx_power);

  void PatchPhy(int device_id, bool isAddToPhy, bool isLowEnergy) {
    auto phy_index = (isLowEnergy) ? phy_low_energy_index_ : phy_classic_index_;
    if (isAddToPhy) {
      mTestModel.AddDeviceToPhy(device_id, phy_index);
    } else {
      mTestModel.RemoveDeviceFromPhy(device_id, phy_index);
    }
  }

 private:
  std::unordered_map<size_t, std::shared_ptr<BluetoothChip>> id_to_chip_;

  size_t phy_low_energy_index_;
  size_t phy_classic_index_;

  bool mStarted = false;
  rootcanal::AsyncManager mAsyncManager;

  SimTestModel mTestModel{
      std::bind(&rootcanal::AsyncManager::GetNextUserId, &mAsyncManager),
      std::bind(&rootcanal::AsyncManager::ExecAsync, &mAsyncManager,
                std::placeholders::_1, std::placeholders::_2,
                std::placeholders::_3),
      std::bind(&rootcanal::AsyncManager::ExecAsyncPeriodically, &mAsyncManager,
                std::placeholders::_1, std::placeholders::_2,
                std::placeholders::_3, std::placeholders::_4),
      std::bind(&rootcanal::AsyncManager::CancelAsyncTasksFromUser,
                &mAsyncManager, std::placeholders::_1),
      std::bind(&rootcanal::AsyncManager::CancelAsyncTask, &mAsyncManager,
                std::placeholders::_1),
      [this](const std::string & /* server */, int /* port */,
             rootcanal::Phy::Type /* phy_type */) { return nullptr; }};

  std::string controller_properties_;
};

class BluetoothChip : public controller::Chip {
 public:
  explicit BluetoothChip(BluetoothChipEmulatorImpl *chip_emulator,
                         std::shared_ptr<rootcanal::HciSniffer> sniffer,
                         int device_index)
      : sniffer(std::move(sniffer)),
        chip_emulator(chip_emulator),
        device_index(device_index) {}

  ~BluetoothChip() {}

  void Reset() override {
    controller::Chip::Reset();
    model::Chip model;
    model.mutable_bt()->mutable_classic()->set_state(State::ON);
    model.mutable_bt()->mutable_low_energy()->set_state(State::ON);
    model.set_capture(State::OFF);
    Patch(model);
  }

  void Patch(const model::Chip &request) override {
    controller::Chip::Patch(request);

    auto &model = Model();

    // Patch packet capture
    if (changedState(model.capture(), request.capture())) {
      model.set_capture(request.capture());
      bool isOn = request.capture() == State::ON;
      SetPacketCapture(isOn);
    }

    // Low_energy radio state
    auto request_state = request.bt().low_energy().state();
    auto *le = model.mutable_bt()->mutable_low_energy();
    if (changedState(le->state(), request_state)) {
      le->set_state(request_state);
      chip_emulator->PatchPhy(device_index, request_state == State::ON, true);
    }
    // Classic radio state
    request_state = request.bt().classic().state();
    auto *classic = model.mutable_bt()->mutable_classic();
    if (changedState(classic->state(), request_state)) {
      classic->set_state(request_state);
      chip_emulator->PatchPhy(device_index, request_state == State::ON, false);
    }
  }

  void Remove() override {
    auto &model = DeviceModel();
    BtsLog("Removing HCI chip for %s", model.name().c_str());
    // NOTE: OnConnectionClosed removes the device from the rootcanal testmodel,
    // so the only cleanup is in the Chip class.
    controller::Chip::Remove();
  }

  void IncrTx(rootcanal::Phy::Type phy_type) {
    if (phy_type == rootcanal::Phy::Type::LOW_ENERGY) {
      auto *low_energy = Model().mutable_bt()->mutable_low_energy();
      low_energy->set_tx_count(low_energy->tx_count() + 1);
    } else {
      auto *classic = Model().mutable_bt()->mutable_classic();
      classic->set_tx_count(classic->tx_count() + 1);
    }
  }

  void IncrRx(rootcanal::Phy::Type phy_type) {
    if (phy_type == rootcanal::Phy::Type::LOW_ENERGY) {
      auto *low_energy = Model().mutable_bt()->mutable_low_energy();
      low_energy->set_rx_count(low_energy->rx_count() + 1);
    } else {
      auto *classic = Model().mutable_bt()->mutable_classic();
      classic->set_rx_count(classic->rx_count() + 1);
    }
  }

 private:
  bool changedState(State a, State b) {
    return (b != State::UNKNOWN && a != b);
  }

  void SetPacketCapture(bool isOn) {
    if (!isOn) {
      sniffer->SetOutputStream(nullptr);
      return;
    }
    // TODO: make multi-os
    // Filename: emulator-5554-hci.pcap
    auto &model = DeviceModel();
    auto filename = "/tmp/" + model.name() + "-hci.pcap";
    for (auto i = 0; netsim::filesystem::exists(filename); ++i) {
      filename = "/tmp/" + model.name() + "-hci-" + std::to_string(i) + ".pcap";
    }
    auto file = std::make_shared<std::ofstream>(filename, std::ios::binary);
    sniffer->SetOutputStream(file);
  }

  std::shared_ptr<rootcanal::HciSniffer> sniffer;
  BluetoothChipEmulatorImpl *chip_emulator;
  int device_index;
};

std::shared_ptr<BluetoothChip> BluetoothChipEmulatorImpl::Get(int device_id) {
  return id_to_chip_[device_id];
}

void BluetoothChipEmulatorImpl::Remove(int device_id) {
  // clear the shared pointer
  id_to_chip_[device_id] = nullptr;
  mTestModel.RemoveDevice(device_id);
}

// Rename AddChip(model::Chip, device, transport)

void BluetoothChipEmulatorImpl::AddHciConnection(
    const std::string &name,
    std::shared_ptr<rootcanal::HciTransport> transport) {
  // rewrap the transport to reschedule callbacks to the async manager
  // event thread.
  transport = std::make_shared<SyncTransport>(transport, mAsyncManager);
  // rewrap the transport to include a sniffer
  transport = rootcanal::HciSniffer::Create(transport);
  auto hci_device =
      std::make_shared<rootcanal::HciDevice>(transport, controller_properties_);
  BtsLog("Creating HCI for %s", name.c_str());
  auto device_id = mTestModel.AddHciConnection(hci_device);

  auto sniffer = std::static_pointer_cast<rootcanal::HciSniffer>(transport);

  model::Chip model;
  model.mutable_bt()->mutable_classic()->set_state(State::ON);
  model.mutable_bt()->mutable_low_energy()->set_state(State::ON);
  model.set_capture(State::OFF);

  auto chip = std::make_shared<BluetoothChip>(this, sniffer, device_id);
  auto device = controller::SceneController::Singleton().GetOrCreate(name);
  device->AddChip(device, std::static_pointer_cast<controller::Chip>(chip),
                  model);
  id_to_chip_[device_id] = chip;
}

int8_t BluetoothChipEmulatorImpl::ComputeRssi(int send_id, int recv_id,
                                              int8_t tx_power) {
  auto sender = id_to_chip_[send_id];
  auto receiver = id_to_chip_[recv_id];
  if (!sender || !receiver) {
    // TODO: Add beacon to netsim.
    // BtsLog("GetRssi unknown send or recv id");
    return tx_power;
  }
  auto distance = controller::SceneController::Singleton().GetDistance(
      *(sender->parent), *(receiver->parent));
  return netsim::DistanceToRssi(tx_power, distance);
}

// For accessing the implementation methods from SimPhyLayerFactory
// avoiding forward references.
int8_t ComputeRssi(int send_id, int recv_id, int8_t tx_power) {
  return static_cast<BluetoothChipEmulatorImpl &>(BluetoothChipEmulator::Get())
      .ComputeRssi(send_id, recv_id, tx_power);
}
void IncrTx(uint32_t send_id, rootcanal::Phy::Type phy_type) {
  auto chip =
      static_cast<BluetoothChipEmulatorImpl &>(BluetoothChipEmulator::Get())
          .Get(send_id);
  if (chip) {
    chip->IncrTx(phy_type);
  }
}
void IncrRx(uint32_t receive_id, rootcanal::Phy::Type phy_type) {
  auto chip =
      static_cast<BluetoothChipEmulatorImpl &>(BluetoothChipEmulator::Get())
          .Get(receive_id);
  if (chip) {
    chip->IncrRx(phy_type);
  }
}

}  // namespace

BluetoothChipEmulator &BluetoothChipEmulator::Get() {
  static BluetoothChipEmulatorImpl sSingleton;
  return sSingleton;
}

}  // namespace hci
}  // namespace netsim
