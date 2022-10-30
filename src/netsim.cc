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

#include <getopt.h>

#include "core/server.h"
#ifdef NETSIM_ANDROID_EMULATOR
#include "core/server_rpc.h"
#endif
#include "fe/cli.h"
#include "hci/bluetooth_chip_emulator.h"

// Wireless network simulator for android (and other) emulated devices.

void ArgError(char *argv[], int c) {
  std::cerr << argv[0] << ": invalid option -- " << (char)c << "\n";
  std::cerr << "Try `" << argv[0] << " --help' for more information.\n";
}

int main(int argc, char *argv[]) {
  const char *kShortOpt = "s:dg";
  bool debug = false;
  bool grpc_startup = false;
  std::string fd_startup_str;

  int c;

  while ((c = getopt_long(argc, argv, kShortOpt, nullptr, nullptr)) != -1) {
    switch (c) {
      case 's':
        fd_startup_str = std::string(optarg);
        break;
#ifdef NETSIM_ANDROID_EMULATOR
      case 'g':
        grpc_startup = true;
        break;
#endif
      case 'd':
        debug = true;
        break;

      default:
        ArgError(argv, c);
        return (-2);
    }
  }

  // Daemon mode -- start radio managers
  if (!fd_startup_str.empty() || grpc_startup)
    netsim::hci::BluetoothChipEmulator::Get().Start();

  if (!fd_startup_str.empty()) {
    netsim::StartWithFds(fd_startup_str, debug);
    return -1;
  }

  // get netsim daemon, starting if it doesn't exist
  auto frontend_stub = netsim::NewFrontendStub();
#ifdef NETSIM_ANDROID_EMULATOR
  if (frontend_stub == nullptr) {
    // starts netsim in vhci connection mode
    frontend_stub = netsim::StartWithGrpc(debug);
  }
#endif
  // could not start the server
  if (frontend_stub == nullptr) return (1);

  if (!grpc_startup) {
    std::vector<std::string_view> args(argv + 1, argv + argc);
    netsim::SendCommand(std::move(frontend_stub), args);
  }

  return (0);
}
