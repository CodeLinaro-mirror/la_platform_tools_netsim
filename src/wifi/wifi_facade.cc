// Copyright 2023 The Android Open Source Project
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

#include "wifi/wifi_facade.h"

#include "util/log.h"

namespace netsim::wifi::facade {
namespace {
// To detect bugs of misuse of chip_id more efficiently.
const int kGlobalChipStartIndex = 2000;
}  // namespace

void Reset(uint32_t) { BtsLog("wifi::facade::Reset()"); }
void Remove(uint32_t) { BtsLog("wifi::facade::Remove()"); }
void Patch(uint32_t, const model::Chip::Radio &) {
  BtsLog("wifi::facade::Patch()");
}
model::Chip::Radio Get(uint32_t) {
  BtsLog("wifi::facade::Get()");
  model::Chip::Radio radio;
  return radio;
}
uint32_t Add(uint32_t simulation_device) {
  BtsLog("wifi::facade::Add()");
  static uint32_t global_chip_id = kGlobalChipStartIndex;
  return global_chip_id++;
}

void Start() { BtsLog("wifi::facade::Start()"); }
void Stop() { BtsLog("wifi::facade::Stop()"); }

}  // namespace netsim::wifi::facade
