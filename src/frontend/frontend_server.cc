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
#include "grpcpp/security/server_credentials.h"
#include "grpcpp/server.h"
#include "grpcpp/server_builder.h"
#include "grpcpp/server_context.h"
#include "grpcpp/support/status.h"
#include "util/log.h"

namespace netsim {
namespace {
class FrontendServer final : public frontend::FrontendService::Service {
 public:
  grpc::Status GetVersion(grpc::ServerContext *context,
                          const google::protobuf::Empty *empty,
                          frontend::VersionResponse *reply) {
    reply->set_version("123b");
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

  grpc::Status UpdateDevice(grpc::ServerContext *context,
                            const frontend::UpdateDeviceRequest *request,
                            google::protobuf::Empty *response) {
    auto status = netsim::controller::SceneController::Singleton().UpdateDevice(
        request->device());
    if (!status)
      return grpc::Status(
          grpc::StatusCode::NOT_FOUND,
          "device " + request->device().device_serial() + " not found.");
    return grpc::Status::OK;
  }

  grpc::Status SetPacketCapture(
      grpc::ServerContext *context,
      const frontend::SetPacketCaptureRequest *request,
      google::protobuf::Empty *empty) {
    model::Device device;
    device.set_device_serial(request->device_serial());
    model::Chip chip;
    // Turn on bt packet capture
    chip.set_capture(request->capture() ? model::State::ON : model::State::OFF);
    chip.mutable_bt();
    device.mutable_chips()->Add()->CopyFrom(chip);
    controller::SceneController::Singleton().UpdateDevice(device);
    return grpc::Status::OK;
  }

  grpc::Status Reset(grpc::ServerContext *context,
                     const google::protobuf::Empty *request,
                     google::protobuf::Empty *empty) {
    netsim::controller::SceneController::Singleton().Reset();
    return grpc::Status::OK;
  }
};

FrontendServer service;
}  // namespace

std::pair<std::unique_ptr<grpc::Server>, std::string> RunFrontendServer() {
  grpc::ServerBuilder builder;
  int selected_port;
  builder.AddListeningPort("0.0.0.0:0", grpc::InsecureServerCredentials(),
                           &selected_port);
  builder.RegisterService(&service);
  std::unique_ptr<grpc::Server> server(builder.BuildAndStart());

  BtsLog("Frontend server listening on localhost: %s",
         std::to_string(selected_port).c_str());
  return std::make_pair(std::move(server), std::to_string(selected_port));
}

}  // namespace netsim
