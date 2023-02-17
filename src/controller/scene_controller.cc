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
#include "util/log.h"

namespace netsim {
namespace controller {

/* static */
SceneController &SceneController::Singleton() {
  static SceneController *kInstance = new SceneController();
  return *kInstance;
}

void SceneController::Add(std::shared_ptr<Device> &device) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  devices_.push_back(device);
}

const std::vector<std::shared_ptr<Device>> SceneController::Copy() {
  std::unique_lock<std::mutex> lock(this->mutex_);
  return devices_;
}

std::shared_ptr<Device> SceneController::GetOrCreate(const std::string &name) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  auto device = GetDevice(name);
  if (device != nullptr) {
    return device;
  }
  device = CreateDevice(name);
  devices_.push_back(device);
  return device;
}

void SceneController::RemoveChip(const std::string &name,
                                 common::ChipKind chip_kind,
                                 const std::string &chip_id) {
  for (int d = 0; d < devices_.size(); d++) {
    if (devices_[d]->model.name() == name) {
      if (devices_[d]->RemoveChip(chip_kind, chip_id)) {
        BtsLog("Removing device %s, no more chips", name.c_str());
        // No more chips, deleting device
        devices_.erase(devices_.begin() + d);
      }
      return;
    }
  }
  std::cerr << "Trying to remove chip from unknown device" << std::endl;
}

std::shared_ptr<Device> SceneController::GetDevice(const std::string &name) {
  for (auto device : devices_) {
    if (device->model.name() == name) return device;
  }
  return {nullptr};
}

// Returns a Device shared_ptr or nullptr
std::shared_ptr<Device> SceneController::MatchDevice(const std::string &name) {
  std::shared_ptr<Device> found = nullptr;
  if (name.empty()) {
    return nullptr;
  }
  for (auto &device : devices_) {
    // query && name -> rename, only match by id
    // query && !name -> match by query
    // !query && name -> match by name
    auto pos = device->model.name().find(name);
    if (pos != std::string::npos) {
      // check for multiple matches
      if (found != nullptr) return nullptr;
      found = device;
    }
  }
  return found;
}

// UI requesting a change in device info
bool SceneController::PatchDevice(const model::Device &request) {
  std::unique_lock<std::mutex> lock(this->mutex_);
  if (request.name().empty()) {
    return false;
  }
  auto device = MatchDevice(request.name());
  if (device == nullptr) return false;
  device->Patch(request);
  DeviceNotifyManager::Get().Notify();
  return true;
}

// Euclidian distance between two devices.
float SceneController::GetDistance(const Device &a, const Device &b) {
  return sqrt((pow(a.model.position().x() - b.model.position().x(), 2) +
               pow(a.model.position().y() - b.model.position().y(), 2) +
               pow(a.model.position().z() - b.model.position().z(), 2)));
}

void SceneController::Reset() {
  std::unique_lock<std::mutex> lock(this->mutex_);
  for (auto &device : devices_) {
    device->Reset();
  }
  DeviceNotifyManager::Get().Notify();
}

}  // namespace controller
}  // namespace netsim
