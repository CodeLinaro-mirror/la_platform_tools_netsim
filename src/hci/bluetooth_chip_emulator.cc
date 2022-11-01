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

#include "hci/bluetooth_chip_emulator.h"

#include <cassert>
#include <filesystem>
#include <iostream>
#include <memory>
#include <utility>

#include "controller/chip.h"
#include "controller/scene_controller.h"
#include "model/devices/link_layer_socket_device.h"
#include "model/hci/hci_sniffer.h"
#include "model/hci/hci_socket_transport.h"
#include "model/setup/async_manager.h"
#include "model/setup/test_command_handler.h"
#include "model/setup/test_model.h"
#include "packet/raw_builder.h"  // for RawBuilder
#include "util/log.h"
#include "util/ranging.h"

namespace netsim {
namespace hci {
namespace {

int8_t ComputeRssi(int send_id, int recv_id, int8_t tx_power);

class SimPhyLayerFactory : public rootcanal::PhyLayerFactory {
  // for constructor inheritance
  using PhyLayerFactory::PhyLayerFactory;

  // Overrides Send in PhyLayerFactory to rewrite rssi in packets
  void Send(::model::packets::LinkLayerPacketView packet, uint32_t id,
            uint32_t device_id) override {
    if (packet.GetType() != ::model::packets::PacketType::RSSI_WRAPPER) {
      rootcanal::PhyLayerFactory::Send(packet, id, device_id);
      return;
    }
    auto rssi_wrapper = ::model::packets::RssiWrapperView::Create(packet);
    // Need to call IsValid() before GetRssi. Hit assert if debug build
    if (!rssi_wrapper.IsValid()) {
      assert(rssi_wrapper.IsValid());
      rootcanal::PhyLayerFactory::Send(packet, id, device_id);
      return;
    }
    auto tx_power = rssi_wrapper.GetRssi();

    for (const auto &recv_phy : phy_layers_) {
      if (id == recv_phy->GetId()) continue;
      int8_t rssi = ComputeRssi(device_id, recv_phy->GetDeviceId(), tx_power);
      // Simply changes the rssi value in the LinkLayerPacketView (!)
      auto rssi_builder = ::model::packets::RssiWrapperBuilder::Create(
          rssi_wrapper.GetSourceAddress(), rssi_wrapper.GetDestinationAddress(),
          rssi,
          std::make_unique<::bluetooth::packet::RawBuilder>(
              std::vector<uint8_t>(rssi_wrapper.GetPayload().begin(),
                                   rssi_wrapper.GetPayload().end())));
      auto bytes = std::make_shared<std::vector<uint8_t>>();
      bluetooth::packet::BitInserter i(*bytes);
      bytes->reserve(rssi_builder->size());
      rssi_builder->Serialize(i);
      auto packet_view =
          ::bluetooth::packet::PacketView<bluetooth::packet::kLittleEndian>(
              bytes);
      auto link_layer_packet_view =
          ::model::packets::LinkLayerPacketView::Create(packet_view);
      assert(link_layer_packet_view.IsValid());
      // Send the re-written packet to all phys (devices)
      recv_phy->Receive(link_layer_packet_view);
    }
  }
};

class SimTestModel : public rootcanal::TestModel {
  // for constructor inheritance
  using rootcanal::TestModel::TestModel;

  std::unique_ptr<rootcanal::PhyLayerFactory> CreatePhy(
      rootcanal::Phy::Type phy_type, size_t phy_index) override {
    return std::make_unique<SimPhyLayerFactory>(phy_type, phy_index);
  };
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
  void Start() override {
    if (mStarted) return;

    phy_low_energy_index_ = mTestModel.AddPhy(rootcanal::Phy::Type::LOW_ENERGY);
    phy_classic_index_ = mTestModel.AddPhy(rootcanal::Phy::Type::BR_EDR);

    // TODO: remove testCommands
    auto testCommands = rootcanal::TestCommandHandler(mTestModel);
    testCommands.RegisterSendResponse([](const std::string &) {});
    testCommands.SetTimerPeriod({"5"});
    testCommands.StartTimer({});
    testCommands.FromFile(mCmdFile);

    mStarted = true;
  };

  void AddHciConnection(
      const std::string &serial,
      std::shared_ptr<rootcanal::HciTransport> transport) override;

  void Remove(int device_index);

  // Resets the root canal library.
  // TODO: rename to Reset()
  void Close() override {
    mTestModel.Reset();
    mStarted = false;
  }

  int8_t ComputeRssi(int send_id, int recv_id, int8_t tx_power) {
    auto sender = id_to_chip_[send_id];
    auto receiver = id_to_chip_[recv_id];
    if (!sender || !receiver) {
      BtsLog("GetRssi unknown send or recv id");
      return tx_power;
    }
    auto distance = controller::SceneController::Singleton().GetDistance(
        *(sender->parent), *(receiver->parent));
    return netsim::DistanceToRssi(tx_power, distance);
  }

  void UpdatePhy(int device_id, bool isAddToPhy, bool isLowEnergy) {
    auto phy_index = (isLowEnergy) ? phy_low_energy_index_ : phy_classic_index_;
    if (isAddToPhy) {
      mTestModel.AddDeviceToPhy(device_id, phy_index);
    } else {
      mTestModel.DelDeviceFromPhy(device_id, phy_index);
    }
  }

 private:
  std::vector<std::shared_ptr<controller::Chip>> id_to_chip_;

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

  std::string mControllerProperties;
  std::string mCmdFile;
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
    model.mutable_bt()->mutable_classic()->set_state(model::State::ON);
    model.mutable_bt()->mutable_low_energy()->set_state(model::State::ON);
    model.set_capture(model::State::OFF);
    Update(model);
  }

  void Update(const model::Chip &request) override {
    controller::Chip::Update(request);

    auto model = Model();

    // Update packet capture
    if (changedState(model.capture(), request.capture())) {
      model.set_capture(request.capture());
      bool isOn = request.capture() == model::State::ON;
      SetPacketCapture(isOn);
    }

    // Low_energy radio state
    auto request_state = request.bt().low_energy().state();
    auto le = model.mutable_bt()->mutable_low_energy();
    if (changedState(le->state(), request_state)) {
      le->set_state(request_state);
      chip_emulator->UpdatePhy(device_index, request_state == model::State::ON,
                               true);
    }
    // Classic radio state
    request_state = request.bt().classic().state();
    auto classic = model.mutable_bt()->mutable_classic();
    if (changedState(classic->state(), request_state)) {
      classic->set_state(request_state);
      chip_emulator->UpdatePhy(device_index, request_state == model::State::ON,
                               false);
    }
  }

  void Remove() override {
    controller::Chip::Remove();
    std::cerr << "Deleting bluetooth chip." << std::endl;
    SetPacketCapture(false);
    sniffer.reset();
    chip_emulator->Remove(device_index);
    chip_emulator = nullptr;
  }

 private:
  bool changedState(model::State a, model::State b) {
    return (b != model::State::UNKNOWN && a != b);
  }

  void SetPacketCapture(bool isOn) {
    if (!isOn) {
      sniffer->SetOutputStream(nullptr);
      return;
    }
    // TODO: make multi-os
    // Filename: emulator-5554-hci.pcap
    auto model = DeviceModel();
    auto filename = "/tmp/" + model.device_serial() + "-hci.pcap";
    for (auto i = 0; std::filesystem::exists(filename); ++i) {
      filename = "/tmp/" + model.device_serial() + "-hci-" + std::to_string(i) +
                 ".pcap";
    }
    auto file = std::make_shared<std::ofstream>(filename, std::ios::binary);
    sniffer->SetOutputStream(file);
  }

  std::shared_ptr<rootcanal::HciSniffer> sniffer;
  BluetoothChipEmulatorImpl *chip_emulator;
  int device_index;
};

void BluetoothChipEmulatorImpl::Remove(int device_id) {
  // clear the shared pointer
  id_to_chip_[device_id] = nullptr;
  mTestModel.Del(device_id);
}

// Rename AddChip(model::Chip, device, transport)

void BluetoothChipEmulatorImpl::AddHciConnection(
    const std::string &serial,
    std::shared_ptr<rootcanal::HciTransport> transport) {
  // rewrap the transport to include a sniffer
  transport = rootcanal::HciSniffer::Create(transport);
  auto hci_device =
      std::make_shared<rootcanal::HciDevice>(transport, mControllerProperties);
  std::cerr << "creating device: " << std::endl;
  auto device_id = mTestModel.AddHciConnection(hci_device);

  auto sniffer = std::static_pointer_cast<rootcanal::HciSniffer>(transport);

  model::Chip model;
  model.mutable_bt()->mutable_classic()->set_state(model::State::ON);
  model.mutable_bt()->mutable_low_energy()->set_state(model::State::ON);
  model.set_capture(model::State::OFF);

  //    auto chip = std::shared_ptr<BluetoothChip>(this, sniffer, device_id);
  auto chip = std::make_shared<BluetoothChip>(this, sniffer, device_id);
  auto device = controller::SceneController::Singleton().GetOrCreate(serial);
  device->AddChip(device, std::static_pointer_cast<controller::Chip>(chip),
                  model);
  id_to_chip_[device_id] = chip;
}

// For accessing the implementation method from SimPhyLayerFactory
int8_t ComputeRssi(int send_id, int recv_id, int8_t tx_power) {
  return static_cast<BluetoothChipEmulatorImpl &>(BluetoothChipEmulator::Get())
      .ComputeRssi(send_id, recv_id, tx_power);
}

}  // namespace

BluetoothChipEmulator &BluetoothChipEmulator::Get() {
  static BluetoothChipEmulatorImpl sSingleton;
  return sSingleton;
}

}  // namespace hci
}  // namespace netsim
