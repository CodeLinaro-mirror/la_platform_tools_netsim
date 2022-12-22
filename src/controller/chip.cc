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

#include "controller/chip.h"

#include "model.pb.h"

namespace netsim {
namespace controller {

void Chip::Init(std::shared_ptr<Device> parent, int chip_index) {
  this->parent = std::move(parent);
  this->chip_index = chip_index;
}

model::Chip &Chip::Model() {
  return this->parent->model.mutable_chips()->at(this->chip_index);
}

model::Device &Chip::DeviceModel() { return this->parent->model; }

void Chip::Update(const model::Chip &request) {
  auto &model = Model();
  if (!request.manufacturer().empty()) {
    model.set_manufacturer(request.manufacturer());
  }
  if (!request.model().empty()) {
    model.set_model(request.model());
  }
}

void Chip::Remove() {}

void Chip::Reset() {
  // Nothing to reset
}

bool Chip::KeyComp(const model::Chip &other_model) {
  auto &model = Model();
  return model.chip_case() == other_model.chip_case() &&
         model.chip_id() == other_model.chip_id();
}
}  // namespace controller
}  // namespace netsim
