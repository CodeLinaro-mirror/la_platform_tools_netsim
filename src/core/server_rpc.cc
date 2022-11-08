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

#include "fe/cli.h"
#include "fe/frontend_server.h"
#include "frontend.grpc.pb.h"
#include "backend/rpc_hal_transport.h"
#include "util/os_utils.h"

namespace netsim {

std::unique_ptr<frontend::FrontendService::Stub> StartWithGrpc(bool debug) {
  // if debug then always run as server, without fork
  auto pid = debug ? 0 : netsim::osutils::Daemon();
  if (pid == 0) {
    // Connect to all emulator grpc servers
    auto grpc_transport = RpcHalTransport::Create();
    grpc_transport->discover();
    netsim::RunFrontendServer();
    // never happens
    return nullptr;
  } else {
    std::cout << "netsim not running; starting now at pid:" << pid << "\n";
    auto frontend_stub = std::unique_ptr<frontend::FrontendService::Stub>{};
    for (int second : {1, 2, 5}) {
      sleep(second);
      frontend_stub = NewFrontendStub();
      if (frontend_stub) break;
    }

    if (frontend_stub != nullptr) {
      std::cout << "netsim started successfully" << std::endl;
    } else {
      std::cerr << "cannot connect to daemon at pid:" << pid << std::endl;
    }
    return frontend_stub;
  }
}

}  // namespace netsim
