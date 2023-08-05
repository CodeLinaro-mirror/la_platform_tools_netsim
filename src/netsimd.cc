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
#endif

#if defined(__linux__)

#ifndef NETSIM_ANDROID_EMULATOR
#include <client/linux/handler/exception_handler.h>
#include <unwindstack/AndroidUnwinder.h>
#endif

#include <execinfo.h>
#include <fmt/format.h>
#include <signal.h>
#include <unistd.h>

#include <cstdio>
#include <cstdlib>
#endif

#include "core/server.h"
#include "frontend/frontend_client_stub.h"
#include "netsim-cxx/src/lib.rs.h"
#include "util/os_utils.h"

// Wireless network simulator for android (and other) emulated devices.

#if defined(__linux__)
#ifndef NETSIM_ANDROID_EMULATOR
bool crash_callback(const void *crash_context, size_t crash_context_size,
                    void * /* context */) {
  std::optional<pid_t> tid;
  std::cerr << "netsimd crash_callback invoked\n";
  if (crash_context_size >=
      sizeof(google_breakpad::ExceptionHandler::CrashContext)) {
    auto *ctx =
        static_cast<const google_breakpad::ExceptionHandler::CrashContext *>(
            crash_context);
    tid = ctx->tid;
    int signal_number = ctx->siginfo.si_signo;
    std::cerr << fmt::format("Process crashed, signal: {}[{}], tid: {}\n",
                             strsignal(signal_number), signal_number, ctx->tid)
                     .c_str();
  } else {
    std::cerr << "Process crashed, signal: unknown, tid: unknown\n";
  }
  unwindstack::AndroidLocalUnwinder unwinder;
  unwindstack::AndroidUnwinderData data;
  if (!unwinder.Unwind(tid, data)) {
    std::cerr << "Unwind failed\n";
    return false;
  }
  std::cerr << "Backtrace:\n";
  for (const auto &frame : data.frames) {
    std::cerr << fmt::format("{}\n", unwinder.FormatFrame(frame)).c_str();
  }
  return true;
}
#else
// Signal handler to print backtraces and then terminate the program.
void SignalHandler(int sig) {
  size_t buffer_size = 20;  // Number of entries in that array.
  void *buffer[buffer_size];

  auto size = backtrace(buffer, buffer_size);
  fprintf(stderr,
          "netsim error: interrupt by signal %d. Obtained %d stack frames:\n",
          sig, size);
  backtrace_symbols_fd(buffer, size, STDERR_FILENO);
  exit(sig);
}
#endif
#endif

constexpr int DEFAULT_HCI_PORT = 6402;

int get_hci_port(int hci_port_flag, uint16_t instance) {
  // The following priorities are used to determine the HCI port number:
  //
  // 1. The CLI flag `-hci_port`.
  // 2. The environment variable `NETSIM_HCI_PORT`.
  // 3. The default value `DEFAULT_HCI_PORT`.
  int hci_port = 0;
  if (hci_port_flag != 0) {
    hci_port = hci_port_flag;
  } else if (auto netsim_hci_port =
                 netsim::osutils::GetEnv("NETSIM_HCI_PORT", "0");
             netsim_hci_port != "0") {
    char *ptr;
    hci_port = strtol(netsim_hci_port.c_str(), &ptr, 10);
  } else {
    hci_port = DEFAULT_HCI_PORT + instance;
  }
  return hci_port;
}

void ArgError(char *argv[], int c) {
  std::cerr << argv[0] << ": invalid option -- " << (char)c << "\n";
  std::cerr << "Try `" << argv[0] << " --help' for more information.\n";
}

int main(int argc, char *argv[]) {
#if defined(__linux__)
#ifndef NETSIM_ANDROID_EMULATOR
  google_breakpad::MinidumpDescriptor descriptor("/tmp");
  google_breakpad::ExceptionHandler eh(descriptor, nullptr, nullptr, nullptr,
                                       true, -1);
  eh.set_crash_handler(crash_callback);
#else
  signal(SIGSEGV, SignalHandler);
#endif
#endif
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
  int hci_port = get_hci_port(hci_port_flag, instance_num);
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
