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
#include <execinfo.h>
#include <signal.h>
#include <unistd.h>

#include <cstdio>
#endif

#include "frontend/cli.h"

// Wireless network simulator for android (and other) emulated devices.

#if defined(__linux__)
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

void ArgError(char *argv[], int c) {
  std::cerr << argv[0] << ": invalid option -- " << (char)c << "\n";
  std::cerr << "Try `" << argv[0] << " --help' for more information.\n";
}

int main(int argc, char *argv[]) {
#if defined(__linux__)
  signal(SIGSEGV, SignalHandler);
#endif

  // get netsim daemon, starting if it doesn't exist
  auto frontend_stub = netsim::NewFrontendStub();
#ifdef NETSIM_ANDROID_EMULATOR
  if (frontend_stub == nullptr) {
    // TODO: starts netsimd like packet streamer client
    std::cerr << "netsimd is not running\n";
  }
#endif
  // could not start the server
  if (frontend_stub == nullptr) return (1);

  std::vector<std::string_view> args(argv + 1, argv + argc);
  netsim::SendCommand(std::move(frontend_stub), args);

  return (0);
}
