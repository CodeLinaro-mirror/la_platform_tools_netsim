/*
 * Copyright 2022 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#pragma once

#include <cstdint>
#include <memory>
#include <string>

#include "grpcpp/server.h"

namespace netsim::server {

struct ServerParams {
  std::string fd_startup_str;
  bool no_cli_ui;
  bool no_web_ui;
  int hci_port;
  int instance_num;
  bool dev;
  int vsock;
};

class GrpcServer {
 public:
  GrpcServer(std::unique_ptr<grpc::Server> server)
      : server(std::move(server)) {}

  void Shutdown() const { server->Shutdown(); }

 private:
  std::unique_ptr<grpc::Server> server;
};

// Run grpc server.
std::unique_ptr<GrpcServer> RunGrpcServerCxx(uint32_t netsim_grpc_port,
                                             bool no_cli_ui,
                                             uint16_t instance_num,
                                             uint16_t vsock);
// Run grpc and http servers.
void Run(ServerParams params);

}  // namespace netsim::server
