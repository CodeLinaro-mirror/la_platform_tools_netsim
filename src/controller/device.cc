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

const std::string GetName(std::string_view device_serial) {
  unsigned int idx =
      std::hash<std::string_view>()(device_serial) % kDeviceNames.size();
  return kDeviceNames.at(idx);
}
}  // namespace

model::Device CreateDevice(std::string_view device_serial) {
  model::Device device;
  device.set_device_serial(stringutils::AsString(device_serial));
  device.set_visible(true);
  // default name
  device.set_name(GetName(device_serial));
  // required sub-messages to simplify ui
  device.mutable_position();
  device.mutable_orientation();
  return device;
}

}  // namespace controller
}  // namespace netsim
