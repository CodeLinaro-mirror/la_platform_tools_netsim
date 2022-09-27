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

#include "fe/frontend_server.cc"

#include <string>

#include "controller/scene_controller.h"
#include "frontend.pb.h"
#include "grpcpp/server_context.h"
#include "grpcpp/support/status.h"
#include "gtest/gtest.h"
#include "model.pb.h"

namespace netsim {
namespace testing {
namespace {

class FrontendServerTest : public ::testing::Test {
 protected:
  // A sample ServerContext for tests.
  grpc::ServerContext context_;
  // An instance of the service under test.
  FrontendServer service_;
};

TEST_F(FrontendServerTest, VerifyVersion) {
  frontend::VersionResponse response;
  grpc::Status status = service_.GetVersion(&context_, {}, &response);
  ASSERT_TRUE(status.ok());
  EXPECT_EQ(response.version(), "123b");
}

TEST_F(FrontendServerTest, SetPositionDevice) {
  netsim::model::Device device_to_add;
  device_to_add.set_device_serial("test-device-serial-for-set-position");
  device_to_add.set_visible(true);
  netsim::controller::SceneController::Singleton().Add(device_to_add);

  google::protobuf::Empty response;
  frontend::SetPositionRequest request;
  request.set_device_serial("test-device-serial-for-set-position");
  request.mutable_position()->set_x(1.1);
  request.mutable_position()->set_y(2.2);
  request.mutable_position()->set_z(3.3);
  grpc::Status status = service_.SetPosition(&context_, &request, &response);
  ASSERT_TRUE(status.ok());
  const auto &scene = netsim::controller::SceneController::Singleton().Get();
  // NOTE: Singleton won't be reset between tests. Need to either deprecate
  // Singleton pattern or provide reset().
  bool found = false;
  for (const auto &device : scene.devices()) {
    if (device.device_serial() == device_to_add.device_serial()) {
      EXPECT_EQ(device.position().x(), device.position().x());
      EXPECT_EQ(device.position().y(), device.position().y());
      EXPECT_EQ(device.position().z(), device.position().z());
      found = true;
      break;
    }
  }
  EXPECT_TRUE(found);
}

TEST_F(FrontendServerTest, SetPositionDeviceNotFound) {
  google::protobuf::Empty response;
  frontend::SetPositionRequest request;
  request.set_device_serial("non-existing-device-serial");
  grpc::Status status = service_.SetPosition(&context_, &request, &response);
  ASSERT_FALSE(status.ok());
  EXPECT_EQ(status.error_code(), grpc::StatusCode::NOT_FOUND);
}

}  // namespace
}  // namespace testing
}  // namespace netsim
