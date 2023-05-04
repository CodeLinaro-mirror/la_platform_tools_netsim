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
#include <cstddef>
#include <iostream>
#include <mutex>
#include <optional>
#include <thread>
#ifdef _WIN32
#include <Windows.h>
#else
#include <unistd.h>
#endif

#include "aemu/base/process/Command.h"
#include "android/base/system/System.h"
#include "android/emulation/control/interceptor/MetricsInterceptor.h"
#include "grpcpp/channel.h"
#include "grpcpp/create_channel.h"
#include "grpcpp/security/credentials.h"
#include "util/log.h"
#include "util/os_utils.h"

using android::control::interceptor::MetricsInterceptorFactory;

namespace netsim::packet {
namespace {

const std::chrono::duration kConnectionDeadline = std::chrono::seconds(1);
std::string custom_packet_stream_endpoint = "";
std::shared_ptr<grpc::Channel> packet_stream_channel;
std::mutex channel_mutex;

std::shared_ptr<grpc::Channel> CreateGrpcChannel() {
  auto endpoint = custom_packet_stream_endpoint;
  if (endpoint.empty()) {
    auto port = netsim::osutils::GetServerAddress();
    if (!port.has_value()) return nullptr;
    endpoint = "localhost:" + port.value();
  }

  if (endpoint.empty()) return nullptr;
  BtsLog("Creating a Grpc channel to %s", endpoint.c_str());

  std::vector<
      std::unique_ptr<grpc::experimental::ClientInterceptorFactoryInterface>>
      interceptors;
  interceptors.emplace_back(std::make_unique<MetricsInterceptorFactory>());
  grpc::ChannelArguments args;
  return grpc::experimental::CreateCustomChannelWithInterceptors(
      endpoint, grpc::InsecureChannelCredentials(), args,
      std::move(interceptors));
}

bool GrpcChannelReady(const std::shared_ptr<grpc::Channel> &channel) {
  if (channel) {
    auto deadline = std::chrono::system_clock::now() + kConnectionDeadline;
    return channel->WaitForConnected(deadline);
  }
  return false;
}

void RunNetsimd(NetsimdOptions options) {
  auto exe = android::base::System::get()->findBundledExecutable("netsimd");
  std::vector<std::string> program_with_args{exe, "-g"};
  if (options.no_cli_ui) program_with_args.push_back("--no_cli_ui");
  if (options.no_web_ui) program_with_args.push_back("--no_web_ui");
  auto cmd = android::base::Command::create(program_with_args);

  auto netsimd = cmd.asDeamon().execute();
  if (netsimd) {
    BtsLog("Running netsimd as pid: %d.", netsimd->pid());
  }
}

}  // namespace

void SetPacketStreamEndpoint(const std::string &endpoint) {
  if (endpoint != "default") custom_packet_stream_endpoint = endpoint;
}

std::shared_ptr<grpc::Channel> GetChannel(NetsimdOptions options) {
  std::lock_guard<std::mutex> lock(channel_mutex);

  bool is_netsimd_started = false;
  for (int second : {1, 2, 4, 8}) {
    if (!packet_stream_channel) packet_stream_channel = CreateGrpcChannel();
    if (GrpcChannelReady(packet_stream_channel)) return packet_stream_channel;

    packet_stream_channel.reset();

    if (!is_netsimd_started && custom_packet_stream_endpoint.empty()) {
      BtsLog("Starting netsim.");
      RunNetsimd(options);
      is_netsimd_started = true;
    }
    BtsLog("Retry connecting to netsim in %d second.", second);
    std::this_thread::sleep_for(std::chrono::seconds(second));
  }

  BtsLog("Unable to get a packet stream channel.");
  return nullptr;
}

std::shared_ptr<grpc::Channel> CreateChannel(NetsimdOptions options) {
  return GetChannel(options);
}

std::shared_ptr<grpc::Channel> CreateChannel(
    std::string _rootcanal_default_commands_file,
    std::string _rootcanal_controller_properties_file) {
  return GetChannel({});
}

}  // namespace netsim::packet
