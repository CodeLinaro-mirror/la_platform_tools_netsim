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

#include "core/server_rpc.h"

#include <iostream>
#include <memory>
#include <thread>

#include "backend/backend_server.h"
#include "frontend/frontend_server.h"
#include "netsim_cxx_generated.h"
#include "util/filesystem.h"
#include "util/ini_file.h"
#include "util/log.h"
#include "util/os_utils.h"

namespace netsim {

void StartWithGrpc(bool debug) {
  BtsLog("starting packet streamer");

  // Run frontend http server.
  std::thread frontend_http_server(RunFrontendHttpServer);

  // Run frontend and backend grpc servers.
  auto [frontend_server, frontend_grpc_port] = netsim::RunFrontendServer();
  auto [backend_server, backend_grpc_port] = netsim::RunBackendServer();

  // Writes grpc ports to ini file.
  auto filepath = osutils::GetDiscoveryDirectory() + netsim::filesystem::slash +
                  "netsim.ini";
  IniFile iniFile(filepath);
  iniFile.Read();
  iniFile.Set("grpc.port", frontend_grpc_port);
  iniFile.Set("grpc.backend.port", backend_grpc_port);
  iniFile.Write();

  frontend_server->Wait();
  backend_server->Wait();
  frontend_http_server.join();
  // never happens
}

}  // namespace netsim
