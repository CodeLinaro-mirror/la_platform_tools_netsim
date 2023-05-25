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

#include "core/server.h"

#include <chrono>
#include <memory>
#include <optional>
#include <string>
#include <thread>

#include "backend/grpc_server.h"
#include "controller/controller.h"
#include "frontend/frontend_server.h"
#include "grpcpp/security/server_credentials.h"
#include "grpcpp/server.h"
#include "grpcpp/server_builder.h"
#include "hci/bluetooth_facade.h"
#include "netsim-cxx/src/lib.rs.h"
#include "util/filesystem.h"
#include "util/ini_file.h"
#include "util/log.h"
#include "util/os_utils.h"
#ifdef _WIN32
#include <Windows.h>
#else
#include <unistd.h>
#endif

namespace netsim::server {

namespace {
constexpr std::chrono::seconds InactivityCheckInterval(5);

std::unique_ptr<grpc::Server> RunGrpcServer(int netsim_grpc_port,
                                            bool no_cli_ui) {
  grpc::ServerBuilder builder;
  int selected_port;
  builder.AddListeningPort("0.0.0.0:" + std::to_string(netsim_grpc_port),
                           grpc::InsecureServerCredentials(), &selected_port);
  if (!no_cli_ui) {
    static auto frontend_service = GetFrontendService();
    builder.RegisterService(frontend_service.release());
  }
  static auto backend_service = GetBackendService();
  builder.RegisterService(backend_service.release());
  builder.AddChannelArgument(GRPC_ARG_ALLOW_REUSEPORT, 0);
  std::unique_ptr<grpc::Server> server(builder.BuildAndStart());
  if (server == nullptr) {
    return nullptr;
  }

  BtsLog("Grpc server listening on localhost: %s",
         std::to_string(selected_port).c_str());

  // Writes grpc port to ini file.
  auto filepath = osutils::GetNetsimIniFilepath();
  IniFile iniFile(filepath);
  iniFile.Read();
  iniFile.Set("grpc.port", std::to_string(selected_port));
  iniFile.Write();

  return std::move(server);
}
}  // namespace

void Run(ServerParams params) {
  // Clear all pcap files in temp directory
  if (netsim::capture::ClearPcapFiles()) {
    BtsLog("netsim generated pcap files in temp directory has been removed.");
  }

  netsim::hci::facade::Start();

#ifndef NETSIM_ANDROID_EMULATOR
  netsim::RunFdTransport(params.fd_startup_str);
#endif

  // Environment variable "NETSIM_GRPC_PORT" is set in google3 forge. If set:
  // 1. Use the fixed port for grpc server.
  // 2. Don't start http server.
  auto netsim_grpc_port = std::stoi(osutils::GetEnv("NETSIM_GRPC_PORT", "0"));

  // Run backend and optionally frontend grpc servers (based on no_cli_ui).
  auto grpc_server = RunGrpcServer(netsim_grpc_port, params.no_cli_ui);
  if (grpc_server == nullptr) {
    BtsLog("Failed to start Grpc server");
    return;
  }

  // no_web_ui disables the web server
  if (netsim_grpc_port == 0 && !params.no_web_ui) {
    // Run frontend http server.
    std::thread(RunHttpServer).detach();
  }

  // Run the socket server.
  BtsLog("RunSocketTransport:%d", params.hci_port);
  RunSocketTransport(params.hci_port);

  while (true) {
    std::this_thread::sleep_for(InactivityCheckInterval);
    if (auto seconds_to_shutdown = netsim::scene_controller::GetShutdownTime();
        seconds_to_shutdown.has_value() &&
        seconds_to_shutdown.value() < std::chrono::seconds(0)) {
      grpc_server->Shutdown();
      BtsLog("Netsim has been shutdown due to inactivity.");
      break;
    }
  }
}

}  // namespace netsim::server
