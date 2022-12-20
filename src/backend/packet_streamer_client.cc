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

#include <chrono>
#include <iostream>
#include <optional>
#include <thread>
#ifdef _WIN32
#include <Windows.h>
#else
#include <unistd.h>
#endif

#include "aemu/base/process/Command.h"
#include "android/base/system/System.h"
#include "grpcpp/channel.h"
#include "grpcpp/create_channel.h"
#include "grpcpp/security/credentials.h"
#include "util/log.h"
#include "util/os_utils.h"

namespace netsim::packet {
namespace {

const std::chrono::duration kConnectionDeadline = std::chrono::seconds(1);

void RunNetsimd(std::string rootcanal_default_commands_file,
                std::string rootcanal_controller_properties_file) {
  auto exe = android::base::System::get()->findBundledExecutable("netsimd");
  auto cmd = android::base::Command::create({exe, "-g"});
  if (!rootcanal_default_commands_file.empty())
    cmd.arg("--rootcanal_default_commands_file=" +
            rootcanal_default_commands_file);
  if (!rootcanal_controller_properties_file.empty())
    cmd.arg("--rootcanal_controller_properties_file=" +
            rootcanal_controller_properties_file);

  auto netsimd = cmd.asDeamon().execute();
  if (netsimd) {
    BtsLog("Running netsimd as pid: %d", netsimd->pid());
  }
}

}  // namespace

std::shared_ptr<grpc::Channel> CreateChannel(
    std::string rootcanal_default_commands_file,
    std::string rootcanal_controller_properties_file) {
  bool start_netsimd = false;
  for (int second : {1, 2, 4, 8}) {
    auto port = netsim::osutils::GetServerAddress();
    if (port.has_value()) {
      auto server = "localhost:" + port.value();
      auto channel =
          grpc::CreateChannel(server, grpc::InsecureChannelCredentials());

      auto deadline = std::chrono::system_clock::now() + kConnectionDeadline;
      if (channel->WaitForConnected(deadline)) {
        return channel;
      }
    }
    if (!start_netsimd) {
      BtsLog("Starting netsim");
      RunNetsimd(rootcanal_default_commands_file,
                 rootcanal_controller_properties_file);
      start_netsimd = true;
    }
    BtsLog("Retry connecting to netsim in %d second", second);
    std::this_thread::sleep_for(std::chrono::seconds(second));
  }

  BtsLog("Unable to start netsim");
  return nullptr;
}

}  // namespace netsim::packet
