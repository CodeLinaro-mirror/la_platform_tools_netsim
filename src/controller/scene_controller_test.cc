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

#include "controller/scene_controller.h"

#include "gtest/gtest.h"
#include "model.pb.h"

namespace netsim {
namespace testing {
namespace {

TEST(SceneTest, GetTest) {
  const auto &scene = netsim::controller::SceneController::Singleton().Get();
  EXPECT_EQ(scene.devices_size(), 0);
}

TEST(SceneTest, AddDevicesAndGetTest) {
  netsim::model::Device device;
  device.set_visible(true);
  netsim::controller::SceneController::Singleton().Add(device);

  const auto &scene = netsim::controller::SceneController::Singleton().Get();
  EXPECT_EQ(scene.devices_size(), 1);
  EXPECT_EQ(scene.devices(0).visible(), true);
}

}  // namespace
}  // namespace testing
}  // namespace netsim
