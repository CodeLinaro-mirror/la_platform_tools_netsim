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

#include "controller/scene_controller.h"

#include <cmath>
#include <cstddef>

#include "controller/device_notify_manager.h"
#include "hci/hci_chip_emulator.h"

namespace netsim {
namespace controller {

/* static */
SceneController &SceneController::Singleton() {
  static SceneController *kInstance = new SceneController();
  return *kInstance;
}

void SceneController::Add(netsim::model::Device &device) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  scene_.add_devices()->CopyFrom(device);
}

const netsim::model::Scene SceneController::Copy() {
  std::unique_lock<std::mutex> lock(this->mutex_);
  return scene_;
}

bool SceneController::SetPosition(const std::string &device_serial,
                                  const netsim::model::Position &position) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  for (auto &device : *scene_.mutable_devices()) {
    if (device.device_serial() == device_serial) {
      device.mutable_position()->CopyFrom(position);
      DeviceNotifyManager::Get().Notify();
      return true;
    }
  }
  return false;
}
// Common update to Phy states used by frontend and Chip Facades
void UpdateRadioInternal(netsim::model::Device &device,
                         const netsim::model::ChipPhyState &radio_state,
                         bool insert) {
  // Phys are stored as in a table, devices have a small number
  for (auto mutable_radio_state : *device.mutable_radio_states()) {
    if (mutable_radio_state.radio() == radio_state.radio()) {
      mutable_radio_state.set_state(radio_state.state());
      return;
    }
  }
  if (insert) {
    // The radio was not found, add a new radio state if called
    // from the Chip Facade.
    auto rs = device.mutable_radio_states()->Add();
    rs->CopyFrom(radio_state);
  }
}

// Internal from Radio Facade
void SceneController::UpdateRadio(const std::string &device_serial,
                                  netsim::model::PhyKind radio,
                                  netsim::model::PhyState state) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  netsim::model::ChipPhyState radio_state;
  radio_state.set_radio(radio);
  radio_state.set_state(state);
  for (auto &device : *scene_.mutable_devices()) {
    if (device.device_serial() == device_serial) {
      UpdateRadioInternal(device, radio_state, true);
      break;
    }
  }
}

// Returns a Mutable Device message or nullptr
netsim::model::Device *SceneController::MatchDevice(const std::string &serial,
                                                    const std::string &name) {
  netsim::model::Device *found = nullptr;
  if (serial.empty() && name.empty()) {
    return nullptr;
  }
  for (netsim::model::Device &device : *scene_.mutable_devices()) {
    // serial && name -> rename, only match by serial
    // serial && !name -> match by serial
    // !serial && name -> match by name
    auto pos = (serial.empty()) ? device.name().find(name)
                                : device.device_serial().find(serial);
    if (pos != std::string::npos) {
      // check for multiple matches
      if (found != nullptr) return nullptr;
      found = &device;
    }
  }
  return found;
}

// UI requesting a change in device info
bool SceneController::UpdateDevice(
    const netsim::model::Device &updated_device) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  if (updated_device.device_serial().empty()) {
    return false;
  }
  netsim::model::Device *device =
      MatchDevice(updated_device.device_serial(), updated_device.name());
  if (device == nullptr) return false;
  if (updated_device.has_position()) {
    device->mutable_position()->CopyFrom(updated_device.position());
  }
  if (updated_device.has_orientation()) {
    device->mutable_orientation()->CopyFrom(updated_device.orientation());
  }
  if (!updated_device.name().empty()) {
    device->set_name(updated_device.name());
  }
  if (updated_device.radio_states().size()) {
    for (auto &radio_state : updated_device.radio_states()) {
      hci::ChipEmulator::Get().SetDeviceRadio(
          device->device_serial(), radio_state.radio(), radio_state.state());
      UpdateRadioInternal(*device, radio_state, false);
    }
  }
  if (updated_device.radio_ranges().size()) {
    // TODO push to model and chip emulators
  }
  DeviceNotifyManager::Get().Notify();
  return true;
}

std::optional<float> SceneController::GetDistance(
    const std::string &device_serial_a, const std::string &device_serial_b) {
  auto a = std::find_if(scene_.devices().begin(), scene_.devices().end(),
                        [device_serial_a](const model::Device &d) {
                          return d.device_serial() == device_serial_a;
                        });
  if (a == std::end(scene_.devices())) return {};
  auto b = std::find_if(scene_.devices().begin(), scene_.devices().end(),
                        [device_serial_b](const model::Device &d) {
                          return d.device_serial() == device_serial_b;
                        });
  if (b == std::end(scene_.devices())) return {};
  return sqrt((pow(a->position().x() - b->position().x(), 2) +
               pow(a->position().y() - b->position().y(), 2) +
               pow(a->position().z() - b->position().z(), 2)));
}

}  // namespace controller
}  // namespace netsim
