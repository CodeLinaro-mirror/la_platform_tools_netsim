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

#include <memory>
#include <string>
#include <thread>

#include "backend/fd_startup.h"
#include "fe/frontend_server.h"
#ifndef NETSIM_ANDROID_EMULATOR
#include "fe/http_server.h"
#endif

namespace netsim {

void StartWithFds(const std::string &startup_str, bool debug) {
  std::unique_ptr<hci::FdStartup> fds = hci::FdStartup::Create();
  fds->Connect(startup_str);

#ifndef NETSIM_ANDROID_EMULATOR
  std::thread http_server(netsim::http::RunHttpServer);
#endif

  // running the frontend server blocks
  netsim::RunFrontendServer();
}

}  // namespace netsim
