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

#include "backend/rpc_hal_transport.h"
#include "frontend/frontend_server.h"

namespace netsim {

void StartWithGrpc(bool debug) {
  std::cout << "netsim starting packet streamer\n";
  // Connect to all emulator grpc servers
  auto grpc_transport = RpcHalTransport::Create();
  grpc_transport->discover();
  netsim::RunFrontendServer();
  // never happens
}

}  // namespace netsim
