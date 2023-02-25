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

#include "controller/device.h"

#include <string>
#include <string_view>
#include <vector>

#include "model.pb.h"
#include "util/log.h"
#include "util/os_utils.h"
#include "util/string_utils.h"

namespace netsim {
namespace controller {
namespace {

// common_typos_disable
const std::vector<std::string> kDeviceNames{
    "Bear", "Boar", "Buck", "Bull", "Calf", "Cavy", "Colt", "Cony", "Coon",
    "Dauw", "Deer", "Dieb", "Douc", "Dzho", "Euro", "Eyra", "Fawn", "Foal",
    "Gaur", "Gilt", "Goat", "Guib", "Gyal", "Hare", "Hart", "Hind", "Hogg",
    "Ibex", "Joey", "Jomo", "Kine", "Kudu", "Lamb", "Lion", "Maki", "Mara",
    "Mare", "Mico", "Mink", "Moco", "Mohr", "Moke", "Mole", "Mona", "Mule",
    "Musk", "Napu", "Neat", "Nowt", "Oont", "Orca", "Oryx", "Oxen", "Paca",
    "Paco", "Pard", "Peba", "Pika", "Pudu", "Puma", "Quey", "Roan", "Runt",
    "Rusa", "Saki", "Seal", "Skug", "Sore", "Tait", "Tegg", "Titi", "Unau",
    "Urus", "Urva", "Vari", "Vole", "Wolf", "Zati", "Zebu", "Zobo", "Zobu"};

const std::string GetName(std::string_view device_name) {
  unsigned int idx =
      std::hash<std::string_view>()(device_name) % kDeviceNames.size();
  return kDeviceNames.at(idx);
}
}  // namespace

void Device::Patch(const model::Device &request) {
  if (request.has_position()) {
    this->model.mutable_position()->CopyFrom(request.position());
  }
  if (request.has_orientation()) {
    this->model.mutable_orientation()->CopyFrom(request.orientation());
  }
  if (!request.name().empty()) {
    this->model.set_name(request.name());
  }
  for (auto &request_chip_model : request.chips()) {
    auto found = false;
    for (auto &chip : chips) {
      if (chip->KeyComp(request_chip_model)) {
        chip->Patch(request_chip_model);
        found = true;
        break;
      }
    }
    if (!found) {
      BtsLog("Unknown chip in update");
    }
  }
}

bool Device::RemoveChip(common::ChipKind chip_kind,
                        const std::string &chip_id) {
  for (int c = 0; c < model.chips().size(); c++) {
    auto c_kind = model.chips().at(c).chip_kind();
    auto c_id = model.chips().at(c).chip_id();
    if (chip_kind == c_kind && chip_id == c_id) {
      // Entry in chips and model are at same index
      BtsLog("Removing chip kind:%d id:'%s' from %s", c_kind, c_id.c_str(),
             model.name().c_str());
      chips[c]->Remove();
      chips.erase(chips.begin() + c);
      model.mutable_chips()->DeleteSubrange(c, 1);
      return chips.size() == 0;
    }
  }
  BtsLog("Trying to remove unknown chip");
  return chips.size() == 0;
}

void Device::AddChip(std::shared_ptr<Device> device, std::shared_ptr<Chip> chip,
                     const model::Chip &chip_model) {
  for (auto &c : chips) {
    if (c->KeyComp(chip_model)) {
      BtsLog("Trying to add a duplicate chip, skipping!");
      return;
    }
  }
  chip->Init(std::move(device), chips.size());
  model.mutable_chips()->Add()->CopyFrom(chip_model);
  chips.push_back(std::move(chip));
}
void Device::Reset() {
  this->model.set_visible(true);
  this->model.mutable_position()->Clear();
  this->model.mutable_orientation()->Clear();
  // TODO: Reset chips and radios.
}

std::shared_ptr<Device> CreateDevice(std::string_view name) {
  model::Device model;
  model.set_name(stringutils::AsString(name));
  model.set_visible(true);
  // required sub-messages to simplify ui
  model.mutable_position();
  model.mutable_orientation();
  return std::make_shared<Device>(model);
}

}  // namespace controller
}  // namespace netsim
