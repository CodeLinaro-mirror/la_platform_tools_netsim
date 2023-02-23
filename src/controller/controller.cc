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

#include "controller/controller.h"

#include <google/protobuf/empty.pb.h>
#include <google/protobuf/util/json_util.h>

#include <cstdint>
#include <string>

#include "common.pb.h"
#include "controller/scene_controller.h"
#include "frontend.pb.h"

namespace netsim::scene_controller {

unsigned int PatchDevice(const std::string &request, std::string &response,
                         std::string &error_message) {
  frontend::PatchDeviceRequest request_proto;
  google::protobuf::util::JsonStringToMessage(request, &request_proto);
  google::protobuf::Empty response_proto;

  auto status = netsim::controller::SceneController::Singleton().PatchDevice(
      request_proto.device());
  if (!status) {
    error_message = "device_serial not found: " + request_proto.device().name();
    return HTTP_STATUS_BAD_REQUEST;
  }

  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return HTTP_STATUS_OK;
}

unsigned int GetDevices(const std::string &request, std::string &response,
                        std::string &error_message) {
  frontend::GetDevicesResponse response_proto;
  auto scene = netsim::controller::SceneController::Singleton().Get();
  for (const auto &device : scene.devices())
    response_proto.add_devices()->CopyFrom(device);
  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return HTTP_STATUS_OK;
}

void RemoveChip(uint32_t device_id, uint32_t chip_id) {
  netsim::controller::SceneController::Singleton().RemoveChip(device_id,
                                                              chip_id);
}

std::tuple<uint32_t, uint32_t, uint32_t> AddChip(
    const std::string &guid, const std::string &device_name,
    common::ChipKind chip_kind, const std::string &chip_name,
    const std::string &manufacturer, const std::string &product_name) {
  return netsim::controller::SceneController::Singleton().AddChip(
      guid, device_name, chip_kind, chip_name, manufacturer, product_name);
}

float GetDistance(uint32_t device_id, uint32_t other_device_id) {
  return netsim::controller::SceneController::Singleton().GetDistance(
      device_id, other_device_id);
}

}  // namespace netsim::scene_controller
