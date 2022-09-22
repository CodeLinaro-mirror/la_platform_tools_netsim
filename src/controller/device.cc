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

#include "model.pb.h"
#include "util/os_utils.h"
#include "util/string_utils.h"

namespace netsim {
namespace controller {

model::Device CreateDevice(std::string_view device_serial) {
  model::Device device;
  device.set_device_serial(stringutils::AsString(device_serial));
  device.set_visible(true);
  return device;
}

}  // namespace controller
}  // namespace netsim
