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
#include "backend/packet_streamer_client.h"

#include "aemu/base/process/Process.h"
#include "gtest/gtest.h"
#include "packet_streamer.grpc.pb.h"
#include "packet_streamer.pb.h"
#include "startup.pb.h"

namespace netsim::testing {
namespace {

class PacketStreamerClientTest : public ::testing::Test {
 private:
  void TearDown() override {
    // Kill netsimd.
    for (auto &process : android::base::Process::fromName("netsimd")) {
      process->terminate();
    }
  }
};

TEST_F(PacketStreamerClientTest, CreateChannelTest) {
  auto channel = packet::CreateChannel();
  ASSERT_TRUE(channel != nullptr);
}

TEST_F(PacketStreamerClientTest, CreateChannelLoopBackTest) {
  auto channel = packet::CreateChannel();
  ASSERT_TRUE(channel != nullptr);

  std::unique_ptr<packet::PacketStreamer::Stub> stub =
      packet::PacketStreamer::NewStub(channel);

  ::grpc::ClientContext context;

  packet::PacketRequest initial_request;
  packet::Stream bt_stream = stub->StreamPackets(&context);
  initial_request.mutable_initial_info()->set_serial("emulator-5554");
  initial_request.mutable_initial_info()->mutable_chip()->set_kind(
      netsim::startup::Chip::ChipKind::Chip_ChipKind_BLUETOOTH);
  bt_stream->Write(initial_request);
  packet::PacketResponse response;
  bt_stream->Read(&response);
  // TODO: Valid response after server is implemented.
  ASSERT_FALSE(response.has_packet());
  ASSERT_FALSE(response.has_error());

  ::grpc::ClientContext context2;
  packet::Stream wifi_stream = stub->StreamPackets(&context2);
  initial_request.mutable_initial_info()->set_serial("emulator-5554");
  initial_request.mutable_initial_info()->mutable_chip()->set_kind(
      netsim::startup::Chip::ChipKind::Chip_ChipKind_WIFI);
  wifi_stream->Write(initial_request);
  response.Clear();
  bt_stream->Read(&response);

  // TODO: Valid response after server is implemented.
  ASSERT_FALSE(response.has_packet());
  ASSERT_FALSE(response.has_error());
}

}  // namespace
}  // namespace netsim::testing
