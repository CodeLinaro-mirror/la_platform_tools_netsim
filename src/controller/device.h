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
#include <string_view>

#include "controller/chip.h"
#include "model.pb.h"

namespace netsim {
namespace controller {

class Chip;

class Device {
 public:
  model::Device model;

  void Update(const model::Device &request);

  bool RemoveChip(model::Chip::ChipCase chip_case, const std::string &chip_id);

  void AddChip(std::shared_ptr<Device>, std::shared_ptr<Chip>,
               const model::Chip &);

  explicit Device(const model::Device &model) : model(model) {}
  explicit Device(const std::string &serial) {
    model.set_device_serial(serial);
  }

  void Reset();

 protected:
  std::vector<std::shared_ptr<Chip>> chips;
};

std::shared_ptr<Device> CreateDevice(std::string_view device_serial);

}  // namespace controller
}  // namespace netsim
