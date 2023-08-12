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

#if defined(_WIN32)
#include <msvc-getopt.h>
#else
#include <getopt.h>
#include <unistd.h>
#endif

#include <iostream>

#include "core/server.h"
#include "frontend/frontend_client_stub.h"
#include "netsim-cxx/src/lib.rs.h"
#include "util/os_utils.h"
#include "util/crash_report.h"

// Wireless network simulator for android (and other) emulated devices.

void ArgError(char *argv[], int c) {
  std::cerr << argv[0] << ": invalid option -- " << (char)c << "\n";
  std::cerr << "Try `" << argv[0] << " --help' for more information.\n";
}

int main(int argc, char *argv[]) {
  netsim::SetUpCrashReport();

  bool no_web_ui = false;
  bool no_cli_ui = false;

  const char *kShortOpt = "s:dl";
  const option kLongOptions[] = {
      {"no_cli_ui", no_argument, 0, 'f'},
      {"no_web_ui", no_argument, 0, 'w'},
      {"rootcanal_controller_properties_file", required_argument, 0, 'p'},
      {"hci_port", required_argument, 0, 'b'},
      {"instance", required_argument, 0, 'i'},
      {"instance_num", required_argument, 0, 'I'},
      {"logtostderr", no_argument, 0, 'l'},
  };

  bool dev = false;
  std::string fd_startup_str;
  std::string rootcanal_controller_properties_file;
  int hci_port_flag = 0;
  uint16_t instance_flag = 0;
  bool logtostderr = false;

  int c;

  while ((c = getopt_long(argc, argv, kShortOpt, kLongOptions, nullptr)) !=
         -1) {
    switch (c) {
#ifndef NETSIM_ANDROID_EMULATOR
      case 's':
        fd_startup_str = std::string(optarg);
        break;
#endif
      case 'd':
        dev = true;
        break;

      case 'p':
        rootcanal_controller_properties_file = std::string(optarg);
        break;

      case 'f':
        no_cli_ui = true;
        break;

      case 'w':
        no_web_ui = true;
        break;

      case 'b':
        hci_port_flag = std::atoi(optarg);
        break;

      case 'I':
      case 'i':
        // NOTE: --instance_num flag is used to run multiple netsimd instances.
        instance_flag = std::atoi(optarg);
        std::cerr << "Netsimd instance: " << instance_flag << std::endl;
        break;

      case 'l':
        // Set whether log messages go to stderr instead of logfiles.
        logtostderr = true;
        break;

      default:
        ArgError(argv, c);
        return (-2);
    }
  }
  if (fd_startup_str.empty()) {
#ifndef NETSIM_ANDROID_EMULATOR
    std::cerr << "Failed to start netsim daemon because fd startup flag `-s` "
                 "is empty\n";
    return -1;
#endif
    // NOTE: Redirect stdout and stderr to files only if netsimd is not invoked
    // by Cuttlefish. Some Cuttlefish builds fail when writing logs to files.
    if (!logtostderr)
      netsim::osutils::RedirectStdStream(
          netsim::NetsimdTempDirString().c_str());
  }

  netsim::config::SetDev(dev);
  auto instance_num = netsim::osutils::GetInstance(instance_flag);
  int hci_port = netsim::osutils::GetHciPort(hci_port_flag, instance_num);
  // Daemon mode -- start radio managers
  // get netsim daemon, starting if it doesn't exist
  // Create a frontend grpc client to check if a netsimd is already running.
  auto frontend_stub = netsim::frontend::NewFrontendClient(instance_num);
  if (frontend_stub != nullptr) {
    std::cerr << "Failed to start netsim daemon because a netsim daemon is "
                 "already running\n";
    return -1;
  }

  netsim::server::Run({.fd_startup_str = fd_startup_str,
                       .no_cli_ui = no_cli_ui,
                       .no_web_ui = no_web_ui,
                       .hci_port = hci_port,
                       .instance_num = instance_num,
                       .dev = dev});
  return -1;
}
