/*
 * Copyright 2022 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#pragma once

#include <memory>
#include <optional>
#include <vector>

#include "controller/device.h"
#include "model.pb.h"

namespace netsim {
namespace controller {

class SceneController {
 public:
  SceneController(const SceneController &) = delete;
  SceneController &operator=(const SceneController &) = delete;
  SceneController(SceneController &&) = delete;
  SceneController &operator=(SceneController &&) = delete;

  static SceneController &Singleton();

  std::shared_ptr<Device> GetOrCreate(const std::string &);

  // When a packet stream goes away the chip is removed. When there are no more
  // chips the device is remove.
  void RemoveChip(const std::string &serial, common::ChipKind chip_kind,
                  const std::string &chip_id);

  void Add(std::shared_ptr<Device> &device);

  const std::vector<std::shared_ptr<Device>> Copy();

  bool PatchDevice(const model::Device &);

  float GetDistance(const Device &, const Device &);

  void Reset();

 protected:
  friend class SceneControllerTest;
  friend class FrontendServerTest;

  std::shared_ptr<Device> GetDevice(const std::string &serial);

  std::shared_ptr<Device> MatchDevice(const std::string &query);

 private:
  SceneController() = default;  // Disallow instantiation outside of the class.
  std::vector<std::shared_ptr<Device>> devices_;
  std::mutex mutex_;
};

}  // namespace controller
}  // namespace netsim
