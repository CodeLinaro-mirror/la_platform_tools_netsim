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

#include <memory>

#include "common.pb.h"
#include "controller/controller.h"
#include "controller/device.h"
#include "gtest/gtest.h"
#include "hci/bluetooth_facade.h"
#include "model.pb.h"

namespace netsim {
namespace controller {

class SceneControllerTest : public ::testing::Test {
 protected:
  void SetUp() override { netsim::hci::facade::Start(); }
  void TearDown() { netsim::hci::facade::Stop(); }
  std::shared_ptr<Device> match(const std::string &name) {
    return SceneController::Singleton().MatchDevice(name);
  }
};

TEST_F(SceneControllerTest, GetTest) {
  const auto size = SceneController::Singleton().Get().devices_size();
  EXPECT_EQ(size, 0);
}

#ifdef NETSIM_ANDROID_EMULATOR
TEST_F(SceneControllerTest, AddDevicesAndGetTest) {
  scene_controller::AddChip("a", "name-AddDevicesAndGetTest",
                            common::ChipKind::BLUETOOTH);

  const auto size = SceneController::Singleton().Get().devices_size();
  EXPECT_EQ(size, 1);
}

TEST_F(SceneControllerTest, DeviceConstructorTest) {
  scene_controller::AddChip("unique-id", "name-DeviceConstructorTest",
                            common::ChipKind::BLUETOOTH);
  auto device = match("name-DeviceConstructorTest");

  EXPECT_EQ("name-DeviceConstructorTest", device->Get().name());
  // Test for non-empty position and orientationa
  EXPECT_TRUE(device->Get().has_position());
  EXPECT_TRUE(device->Get().has_orientation());
}

TEST_F(SceneControllerTest, MatchDeviceTest) {
  scene_controller::AddChip("guid:1", "name1", common::ChipKind::BLUETOOTH);
  scene_controller::AddChip("guid:2", "name2", common::ChipKind::BLUETOOTH);
  scene_controller::AddChip("guid:3", "name3", common::ChipKind::BLUETOOTH);

  //  matches with name
  ASSERT_TRUE(match("name1"));
  ASSERT_TRUE(match("name2"));
  ASSERT_TRUE(match("name3"));
  ASSERT_TRUE(match("non-existing-name") == nullptr);
}

TEST_F(SceneControllerTest, ResetTest) {
  auto name = "name-ResetTest";
  auto [device_id, chip_id, _] = scene_controller::AddChip(
      "name-for-reset-test", name, common::ChipKind::BLUETOOTH);
  model::Device model;
  model.set_name(name);
  model.set_visible(false);
  model.mutable_position()->set_x(10.0);
  model.mutable_position()->set_y(20.0);
  model.mutable_position()->set_z(30.0);
  model.mutable_orientation()->set_pitch(1.0);
  model.mutable_orientation()->set_roll(2.0);
  model.mutable_orientation()->set_yaw(3.0);

  auto status = SceneController::Singleton().PatchDevice(model);
  EXPECT_TRUE(status);
  auto device = match(name);
  model = device->Get();
  EXPECT_EQ(model.visible(), false);
  EXPECT_EQ(model.position().x(), 10.0);
  EXPECT_EQ(model.orientation().pitch(), 1.0);

  SceneController::Singleton().Reset();

  device = match(name);
  model = device->Get();

  EXPECT_EQ(model.visible(), true);
  EXPECT_EQ(model.position().x(), 0.0);
  EXPECT_EQ(model.position().y(), 0.0);
  EXPECT_EQ(model.position().z(), 0.0);
  EXPECT_EQ(model.orientation().pitch(), 0.0);
  EXPECT_EQ(model.orientation().roll(), 0.0);
  EXPECT_EQ(model.orientation().yaw(), 0.0);
}
#endif

}  // namespace controller
}  // namespace netsim
