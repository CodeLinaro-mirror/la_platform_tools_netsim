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

#include "util/ini_file.h"
#include "util/log.h"

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
  // $TMPDIR is the temp directory on buildbots.
  const char *test_env_p = std::getenv("TMPDIR");
  if (test_env_p && *test_env_p) {
    return std::filesystem::path(test_env_p);
  }
  const char *env_p = std::getenv(discovery.root_env);
  if (!env_p) {
    BtsLog("No discovery env for %s, using tmp/", discovery.root_env);
    env_p = "/tmp";
  }
  std::filesystem::path path(env_p);
  path.append(discovery.subdir);
  return path;
}

std::optional<std::string> GetServerAddress(bool frontend_server) {
  auto filepath = osutils::GetDiscoveryDirectory().append("netsim.ini");
  if (!std::filesystem::exists(filepath)) {
    BtsLog("Unable to find discovery directory: %s", filepath.c_str());
    return std::nullopt;
  }
  if (!std::filesystem::is_regular_file(filepath)) {
    BtsLog("Not a regular file: %s", filepath.c_str());
    return std::nullopt;
  }
  IniFile iniFile(filepath);
  iniFile.Read();
  return iniFile.Get(frontend_server ? "grpc.port" : "grpc.backend.port");
}
}  // namespace osutils
}  // namespace netsim
