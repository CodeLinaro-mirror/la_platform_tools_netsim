// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#include "util/crash_report.h"

#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>
#if defined(_WIN32)
#include <windows.h>
#endif

#if defined(__linux__)

#ifndef NETSIM_ANDROID_EMULATOR
#include <client/linux/handler/exception_handler.h>
#include <fmt/format.h>
#include <unwindstack/AndroidUnwinder.h>

#include <iostream>
#else
#include <signal.h>
#endif

#include <execinfo.h>
#include <unistd.h>

#include <cstdio>
#include <cstdlib>
#endif

#include "util/filesystem.h"
#include "util/ini_file.h"
#include "util/log.h"

namespace netsim {
namespace {

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

}  // namespace

void SetUpCrashReport() {
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
}

}  // namespace netsim
