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

#include "frontend/frontend_server.h"

#include <iostream>
#include <memory>
#include <string>
#include <utility>

#include "controller/scene_controller.h"
#include "frontend.grpc.pb.h"
#include "frontend.pb.h"
#include "google/protobuf/empty.pb.h"
#include "grpcpp/server_context.h"
#include "grpcpp/support/status.h"
#include "netsim_cxx_generated.h"

namespace netsim {
namespace {
class FrontendServer final : public frontend::FrontendService::Service {
 public:
  grpc::Status GetVersion(grpc::ServerContext *context,
                          const google::protobuf::Empty *empty,
                          frontend::VersionResponse *reply) {
    reply->set_version(std::string(netsim::GetVersion()));
    return grpc::Status::OK;
  }

  grpc::Status GetDevices(grpc::ServerContext *context,
                          const google::protobuf::Empty *empty,
                          frontend::GetDevicesResponse *reply) {
    const auto devices =
        netsim::controller::SceneController::Singleton().Copy();
    for (const auto &device : devices)
      reply->add_devices()->CopyFrom(device->model);
    return grpc::Status::OK;
  }

  grpc::Status PatchDevice(grpc::ServerContext *context,
                           const frontend::PatchDeviceRequest *request,
                           google::protobuf::Empty *response) {
    auto status = netsim::controller::SceneController::Singleton().PatchDevice(
        request->device());
    if (!status)
      return grpc::Status(grpc::StatusCode::NOT_FOUND,
                          "device " + request->device().name() + " not found.");
    return grpc::Status::OK;
  }

  grpc::Status SetPacketCapture(
      grpc::ServerContext *context,
      const frontend::SetPacketCaptureRequest *request,
      google::protobuf::Empty *empty) {
    model::Device device;
    model::Chip chip;
    // Turn on bt packet capture
    chip.set_capture(request->capture() ? model::State::ON : model::State::OFF);
    chip.mutable_bt();
    device.mutable_chips()->Add()->CopyFrom(chip);
    controller::SceneController::Singleton().PatchDevice(device);
    return grpc::Status::OK;
  }

  grpc::Status Reset(grpc::ServerContext *context,
                     const google::protobuf::Empty *request,
                     google::protobuf::Empty *empty) {
    netsim::controller::SceneController::Singleton().Reset();
    return grpc::Status::OK;
  }
};
}  // namespace

std::unique_ptr<frontend::FrontendService::Service> GetFrontendService() {
  return std::make_unique<FrontendServer>();
}

}  // namespace netsim
