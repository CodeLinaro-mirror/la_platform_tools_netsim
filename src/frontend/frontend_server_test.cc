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

// Unit tests for the FrontendServer class.

#include "frontend/frontend_server.cc"

#include <string>

#include "controller/device.h"
#include "controller/scene_controller.h"
#include "frontend.pb.h"
#include "grpcpp/server_context.h"
#include "grpcpp/support/status.h"
#include "gtest/gtest.h"
#include "model.pb.h"

namespace netsim {
namespace controller {

class FrontendServerTest : public ::testing::Test {
 protected:
  // A sample ServerContext for tests.
  grpc::ServerContext context_;
  // An instance of the service under test.
  FrontendServer service_;

  std::shared_ptr<controller::Device> GetDevice(const std::string &serial) {
    return controller::SceneController::Singleton().GetDevice(serial);
  }
};

TEST_F(FrontendServerTest, VerifyVersion) {
  frontend::VersionResponse response;
  grpc::Status status = service_.GetVersion(&context_, {}, &response);
  ASSERT_TRUE(status.ok());
  EXPECT_EQ(response.version(), "123b");
}

TEST_F(FrontendServerTest, UpdateDevicePosition) {
  auto serial = "test-device-serial-for-set-position";
  auto device_to_add = controller::CreateDevice(serial);
  netsim::controller::SceneController::Singleton().Add(device_to_add);

  google::protobuf::Empty response;
  frontend::UpdateDeviceRequest request;
  request.mutable_device()->set_device_serial(serial);
  request.mutable_device()->mutable_position()->set_x(1.1);
  request.mutable_device()->mutable_position()->set_y(2.2);
  request.mutable_device()->mutable_position()->set_z(3.3);
  grpc::Status status = service_.UpdateDevice(&context_, &request, &response);
  ASSERT_TRUE(status.ok());
  const auto &scene = netsim::controller::SceneController::Singleton().Copy();
  // NOTE: Singleton won't be reset between tests. Need to either deprecate
  // Singleton pattern or provide reset().
  bool found = false;
  for (const auto &device :
       netsim::controller::SceneController::Singleton().Copy()) {
    if (device->model.device_serial() == request.device().device_serial()) {
      EXPECT_EQ(device->model.position().x(), request.device().position().x());
      EXPECT_EQ(device->model.position().y(), request.device().position().y());
      EXPECT_EQ(device->model.position().z(), request.device().position().z());
      found = true;
      break;
    }
  }
  EXPECT_TRUE(found);
}

TEST_F(FrontendServerTest, UpdateDevice) {
  auto serial = std::string("serial-for-update");
  auto name = std::string("name-for-update");
  auto device_to_add = controller::CreateDevice(serial);
  device_to_add->model.set_name(name);
  {
    auto chip = device_to_add->model.mutable_chips()->Add();
    chip->mutable_bt()->mutable_classic()->set_state(model::State::ON);
  }
  netsim::controller::SceneController::Singleton().Add(device_to_add);

  frontend::UpdateDeviceRequest request;
  google::protobuf::Empty response;
  request.mutable_device()->set_device_serial(serial);
  request.mutable_device()->set_name(name);
  {
    auto chip = request.mutable_device()->mutable_chips()->Add();
    chip->mutable_bt()->mutable_classic()->set_state(model::State::ON);
  }

  grpc::Status status = service_.UpdateDevice(&context_, &request, &response);
  ASSERT_TRUE(status.ok());
  auto optional_device = GetDevice(serial);
  ASSERT_TRUE(optional_device != nullptr);
  ASSERT_TRUE(optional_device->model.name() == name);
  ASSERT_TRUE(optional_device->model.chips().size() == 1);
  ASSERT_TRUE(optional_device->model.chips().Get(0).chip_case() ==
              model::Chip::ChipCase::kBt);
  ASSERT_TRUE(optional_device->model.chips().Get(0).bt().classic().state() ==
              model::State::ON);
}

TEST_F(FrontendServerTest, UpdateDeviceNotFound) {
  google::protobuf::Empty response;
  frontend::UpdateDeviceRequest request;
  request.mutable_device()->set_device_serial("non-existing-device-serial");
  grpc::Status status = service_.UpdateDevice(&context_, &request, &response);
  ASSERT_FALSE(status.ok());
  EXPECT_EQ(status.error_code(), grpc::StatusCode::NOT_FOUND);
}

}  // namespace controller
}  // namespace netsim
