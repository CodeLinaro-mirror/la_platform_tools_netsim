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

#include "util/os_utils.h"

#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

#include <filesystem>
#include <string>

namespace netsim {
namespace osutils {
namespace {

struct DiscoveryDir {
  const char *root_env;
  const char *subdir;
};

DiscoveryDir discovery {
#if defined(_WIN32)
  "LOCALAPPDATA", "Temp"
#elif defined(__linux__)
  "XDG_RUNTIME_DIR", ""
#elif defined(__APPLE__)
  "HOME", "Library/Caches/TemporaryItems"
#else
#error This platform is not supported.
#endif
};

}  // namespace

std::filesystem::path GetDiscoveryDirectory() {
  std::filesystem::path path(std::getenv(discovery.root_env));
  path.append(discovery.subdir);
  return path;
}

int Daemon() {
  pid_t pid = 0;

  /* fork the child process */
  pid = fork();

  /* return pid or -1 to parent */
  if (pid != 0) {
    return pid;
  }

  // child process continues...

  // disassociate from parent's session
  if (setsid() < 0) {
    exit(EXIT_FAILURE);
  }

  // change to the root directory
  chdir("/");

  // redirect stdin, stdout, and stderr to /dev/null
  int fd = open("/dev/null", O_RDWR | O_CLOEXEC);
  if (fd < 0) exit(EXIT_FAILURE);
  if (dup2(fd, STDIN_FILENO) < 0 || dup2(fd, STDOUT_FILENO) < 0 ||
      dup2(fd, STDERR_FILENO) < 0) {
    close(fd);
    exit(EXIT_FAILURE);
  }
  close(fd);

  // Child process returns 0 to caller
  return 0;
}
}  // namespace osutils
}  // namespace netsim
