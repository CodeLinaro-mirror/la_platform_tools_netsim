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
#include <string>

#include "backend/grpc_server.h"
#include "frontend/frontend_server.h"
#include "grpcpp/security/server_credentials.h"
#include "grpcpp/server.h"
#include "grpcpp/server_builder.h"
#include "netsim-cxx/src/lib.rs.h"
#include "util/ini_file.h"
#include "util/log.h"
#include "util/os_utils.h"
#ifdef _WIN32
#include <Windows.h>
#else
#include <unistd.h>
#endif
#ifndef NETSIM_ANDROID_EMULATOR
#include <sys/socket.h>

// Needs to be below sys/socket.h
#include <linux/vm_sockets.h>
#endif
namespace netsim::server {

namespace {
constexpr std::chrono::seconds InactivityCheckInterval(5);

std::unique_ptr<grpc::Server> RunGrpcServer(int netsim_grpc_port,
                                            bool no_cli_ui, int instance_num,
                                            int vsock) {
  grpc::ServerBuilder builder;
  int selected_port;
  builder.AddListeningPort("0.0.0.0:" + std::to_string(netsim_grpc_port),
                           grpc::InsecureServerCredentials(), &selected_port);
  if (!no_cli_ui) {
    static auto frontend_service = GetFrontendService();
    builder.RegisterService(frontend_service.release());
  }

#ifndef NETSIM_ANDROID_EMULATOR
  if (vsock != 0) {
    std::string vsock_uri =
        "vsock:" + std::to_string(VMADDR_CID_ANY) + ":" + std::to_string(vsock);
    BtsLog("vsock_uri: %s", vsock_uri.c_str());
    builder.AddListeningPort(vsock_uri, grpc::InsecureServerCredentials());
  }
#endif

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
  auto filepath = osutils::GetNetsimIniFilepath(instance_num);
  IniFile iniFile(filepath);
  iniFile.Read();
  iniFile.Set("grpc.port", std::to_string(selected_port));
  iniFile.Write();

  return std::move(server);
}
}  // namespace

std::unique_ptr<GrpcServer> RunGrpcServerCxx(uint32_t netsim_grpc_port,
                                             bool no_cli_ui,
                                             uint16_t instance_num,
                                             uint16_t vsock) {
  auto grpc_server =
      RunGrpcServer(netsim_grpc_port, no_cli_ui, instance_num, vsock);
  if (grpc_server == nullptr) return nullptr;
  return std::make_unique<GrpcServer>(std::move(grpc_server));
}

void Run(ServerParams params) {
  auto rust_service = netsim::CreateService(
      params.fd_startup_str, params.no_cli_ui, params.no_web_ui,
      params.hci_port, params.instance_num, params.dev, params.vsock);
  rust_service->SetUp();

  rust_service->Run();
}

}  // namespace netsim::server