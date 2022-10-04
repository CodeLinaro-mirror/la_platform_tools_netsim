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

#include "hci/hci_chip_emulator.h"

#include <cassert>
#include <filesystem>
#include <iostream>
#include <memory>
#include <utility>

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
      int8_t rssi = ChipEmulator::Get().GetRssi(
          device_id, recv_phy->GetDeviceId(), tx_power);
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

class SimHciDevice : public rootcanal::HciDevice {
 public:
  // for constructor inheritance
  using rootcanal::HciDevice::HciDevice;

  static std::shared_ptr<SimHciDevice> Create(
      std::shared_ptr<rootcanal::HciTransport> transport,
      const std::string &properties_filename) {
    return std::make_shared<SimHciDevice>(transport, properties_filename);
  }

  bool HasPhy(rootcanal::Phy::Type phy_type) {
    for (auto &phy : phy_layers_) {
      if (phy != nullptr && phy->GetType() == phy_type) {
        return true;
      }
    }
    return false;
  }
};

// Stores all the information associated with a connected device
struct ConnectedDevice {
  ConnectedDevice(std::string serial, std::shared_ptr<SimHciDevice> device,
                  size_t device_id,
                  std::shared_ptr<rootcanal::HciSniffer> sniffer)
      : serial(std::move(serial)),
        device(std::move(device)),
        device_id(device_id),
        sniffer(std::move(sniffer)) {}
  std::string serial;
  std::shared_ptr<SimHciDevice> device;
  size_t device_id;
  std::shared_ptr<rootcanal::HciSniffer> sniffer;
};

// Private implementation class for ChipEmulator
//
class ChipEmulatorImpl : public ChipEmulator {
 public:
  ChipEmulatorImpl() {}
  ~ChipEmulatorImpl() {}

  ChipEmulatorImpl(const ChipEmulatorImpl &) = delete;

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

  // Resets the root canal library.
  // TODO: rename to Reset()
  void Close() override {
    mTestModel.Reset();
    mStarted = false;
  }

  // Enable or disable a single phy for a device
  void UpdatePhy(const ConnectedDevice &cd, rootcanal::Phy::Type phy_type,
                 size_t phy_index, netsim::model::RadioState new_state) {
    auto current = cd.device->HasPhy(phy_type);
    using RadioState = netsim::model::RadioState;
    if (current && new_state == RadioState::DOWN) {
      mTestModel.DelDeviceFromPhy(cd.device_id, phy_index);
    } else if (!current && new_state == RadioState::UP) {
      mTestModel.AddDeviceToPhy(cd.device_id, phy_index);
    }
  }

  // Enable or disable the radios for a device
  void SetDeviceRadio(const std::string &serial, netsim::model::Radio radio,
                      netsim::model::RadioState state) override {
    if (auto cd = GetConnectedDevice(serial); cd != nullptr) {
      if (radio == netsim::model::Radio::BLUETOOTH_LOW_ENERGY) {
        UpdatePhy(*cd, rootcanal::Phy::Type::LOW_ENERGY, phy_low_energy_index_,
                  state);
      } else if (radio == netsim::model::Radio::BLUETOOTH_CLASSIC) {
        UpdatePhy(*cd, rootcanal::Phy::Type::BR_EDR, phy_classic_index_, state);
      }
    }
  }

  int8_t GetRssi(int send_id, int recv_id, int8_t tx_power) override {
    auto send_serial = GetSerialFromId(send_id);
    auto recv_serial = GetSerialFromId(recv_id);
    if (send_serial.empty() || recv_serial.empty()) {
      BtsLog("GetRssi unknown send or recv id");
      return tx_power;
    }
    auto distance = controller::SceneController::Singleton().GetDistance(
        send_serial, recv_serial);
    // if a problem return no distance
    return (distance.has_value()) ? netsim::DistanceToRssi(tx_power, *distance)
                                  : tx_power;
  }

  ConnectedDevice const *GetConnectedDevice(const std::string &serial) {
    if (auto iter = devices_.find(serial); iter != devices_.end()) {
      return &iter->second;
    }
    BtsLog("Device not found in SimTestModel for serial %s", serial.c_str());
    return nullptr;
  }

  std::string GetSerialFromId(uint32_t device_id) {
    for (const auto &[key, cd] : devices_) {
      if (cd.device_id == device_id) return key;
    }
    BtsLog("Device not found in SimTestModel for device_id %d", device_id);
    return "";
  }

  void AddHciConnection(
      const std::string &serial,
      std::shared_ptr<rootcanal::HciTransport> transport) override {
    // rewrap the transport to include a sniffer
    transport = rootcanal::HciSniffer::Create(transport);
    auto device = SimHciDevice::Create(transport, mControllerProperties);
    std::cerr << "creating device: " << std::endl;
    auto device_id = mTestModel.AddHciConnection(device);

    // update the scene controller with the radio state for this device

    controller::SceneController::Singleton().UpdateRadio(
        serial, netsim::model::Radio::BLUETOOTH_LOW_ENERGY,
        device->HasPhy(rootcanal::Phy::Type::LOW_ENERGY)
            ? model::RadioState::UP
            : model::RadioState::DOWN);
    controller::SceneController::Singleton().UpdateRadio(
        serial, netsim::model::Radio::BLUETOOTH_CLASSIC,
        device->HasPhy(rootcanal::Phy::Type::BR_EDR) ? model::RadioState::UP
                                                     : model::RadioState::DOWN);

    auto sniffer = std::static_pointer_cast<rootcanal::HciSniffer>(transport);
    devices_.emplace(std::make_pair(
        serial, ConnectedDevice(serial, device, device_id, sniffer)));
  }

  void SetPacketCapture(const std::string &serial, bool onOff) {
    if (serial.empty()) {
      for (const auto &[key, cd] : devices_) {
        SetPacketCaptureInternal(cd, onOff);
      }
      return;
    }
    if (auto cd = GetConnectedDevice(serial); cd != nullptr) {
      SetPacketCaptureInternal(*cd, onOff);
    } else {
      std::cerr << "SetPacketCapture: unknown device " << serial << std::endl;
    }
  }

 private:
  // The connected clients in a map
  std::map<std::string, ConnectedDevice> devices_;

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

  void SetPacketCaptureInternal(const ConnectedDevice &cd, bool onOff) {
    if (!onOff) {
      cd.sniffer->SetOutputStream(nullptr);
      return;
    }

    // TODO: make multi-os
    // Filename: emulator-5554-hci.pcap
    auto filename = "/tmp/" + cd.serial + "-hci.pcap";
    for (auto i = 0; std::filesystem::exists(filename); ++i) {
      filename = "/tmp/" + cd.serial + "-hci-" + std::to_string(i) + ".pcap";
    }
    auto file = std::make_shared<std::ofstream>(filename, std::ios::binary);
    cd.sniffer->SetOutputStream(file);
  }
};

}  // namespace

ChipEmulator &ChipEmulator::Get() {
  static ChipEmulatorImpl sSingleton;
  return sSingleton;
}

}  // namespace hci
}  // namespace netsim
