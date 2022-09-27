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

#include "hci/hci_chip_emulator.h"
#include "util/log.h"

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
const netsim::model::Scene &SceneController::Get() const { return scene_; }

bool SceneController::SetPosition(const std::string &device_serial,
                                  const netsim::model::Position &position) {
  for (auto &device : *scene_.mutable_devices()) {
    if (device.device_serial() == device_serial) {
      device.mutable_position()->CopyFrom(position);
      return true;
    }
  }
  return false;
}

void UpdateRadioInternal(netsim::model::Radios &mutable_radios,
                         netsim::model::Radio radio,
                         netsim::model::RadioState state) {
  switch (radio) {
    case netsim::model::Radio::BLUETOOTH_LOW_ENERGY:
      mutable_radios.set_bluetooth_low_energy(state);
      break;
    case netsim::model::Radio::BLUETOOTH_CLASSIC:
      mutable_radios.set_bluetooth_classic(state);
      break;
    default:
      // do not care about unimplemented radios
      break;
  }
}

// Radio Facade informing a change in radio state
void SceneController::UpdateRadio(const std::string &device_serial,
                                  netsim::model::Radio radio,
                                  netsim::model::RadioState state) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  for (auto &device : *scene_.mutable_devices()) {
    if (device.device_serial() == device_serial) {
      UpdateRadioInternal(*device.mutable_radios(), radio, state);
      break;
    }
  }
}

// UI requesting a change in radios
bool SceneController::SetRadio(const std::string &device_serial,
                               netsim::model::Radio radio,
                               netsim::model::RadioState state) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  for (auto &device : *scene_.mutable_devices()) {
    if (device.device_serial() == device_serial) {
      hci::ChipEmulator::Get().SetDeviceRadio(device_serial, radio, state);
      UpdateRadioInternal(*device.mutable_radios(), radio, state);
      return true;
    }
  }
  return false;
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
