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
#if defined(_WIN32)
#include <windows.h>
#endif

#include <string>

#include "util/filesystem.h"
#include "util/ini_file.h"
#include "util/log.h"

namespace netsim {
namespace osutils {
namespace {

constexpr uint16_t DEFAULT_INSTANCE = 0;
constexpr uint32_t DEFAULT_HCI_PORT = 6402;

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

std::string GetEnv(const std::string &name, const std::string &default_value) {
  auto val = std::getenv(name.c_str());
  if (!val) {
    return default_value;
  }
  return val;
}

std::string GetDiscoveryDirectory() {
  // $TMPDIR is the temp directory on buildbots.
  const char *test_env_p = std::getenv("TMPDIR");
  if (test_env_p && *test_env_p) {
    return std::string(test_env_p);
  }
  const char *env_p = std::getenv(discovery.root_env);
  if (!env_p) {
    BtsLog("No discovery env for %s, using tmp/", discovery.root_env);
    env_p = "/tmp";
  }
  return std::string(env_p) + netsim::filesystem::slash + discovery.subdir;
}

std::string GetNetsimIniFilepath(uint16_t instance_num) {
  auto discovery_dir = GetDiscoveryDirectory();
  // Check if directory has a trailing slash.
  if (discovery_dir.back() != netsim::filesystem::slash.back())
    discovery_dir.append(netsim::filesystem::slash);
  auto filename = (instance_num == 0)
                      ? "netsim.ini"
                      : "netsim_" + std::to_string(instance_num) + ".ini";
  discovery_dir.append(filename);
  return discovery_dir;
}

std::optional<std::string> GetServerAddress(uint16_t instance_num) {
  auto filepath = GetNetsimIniFilepath(instance_num);
  if (!netsim::filesystem::exists(filepath)) {
    BtsLog("Unable to find netsim ini file: %s", filepath.c_str());
    return std::nullopt;
  }
  if (!netsim::filesystem::is_regular_file(filepath)) {
    BtsLog("Not a regular file: %s", filepath.c_str());
    return std::nullopt;
  }
  IniFile iniFile(filepath);
  iniFile.Read();
  return iniFile.Get("grpc.port");
}

bool is_stderr_open() {
  // TODO: Use `is_terminal` method in `IsTerminal` trait in Rust.
#if defined(_WIN32)
  return GetStdHandle(STD_ERROR_HANDLE) != INVALID_HANDLE_VALUE;
#else
  return fcntl(STDERR_FILENO, F_GETFD) != -1;
#endif
}

void RedirectStdStream(std::string netsim_temp_dir) {
  // Check if directory has a trailing slash.
  if (netsim_temp_dir.back() != netsim::filesystem::slash.back())
    netsim_temp_dir.append(netsim::filesystem::slash);
  if (is_stderr_open())
    BtsLog("Redirecting logs to %s", netsim_temp_dir.c_str());
  std::freopen((netsim_temp_dir + "netsim_stdout.log").c_str(), "w", stdout);
  std::freopen((netsim_temp_dir + "netsim_stderr.log").c_str(), "w", stderr);
}

uint16_t GetInstance(uint16_t instance_flag) {
  // The following priorities are used to determine the instance number:
  //
  // 1. The environment variable `NETSIM_INSTANCE`.
  // 2. The CLI flag `--instance`.
  // 3. The default value `DEFAULT_INSTANCE`.
  uint16_t instance = 0;
  if (auto netsim_instance = netsim::osutils::GetEnv("NETSIM_INSTANCE", "");
      netsim_instance != "") {
    char *ptr;
    instance = strtol(netsim_instance.c_str(), &ptr, 10);
  } else if (instance_flag != 0) {
    instance = instance_flag;
  } else {
    instance = DEFAULT_INSTANCE;
  }
  return instance;
}

uint32_t GetHciPort(uint32_t hci_port_flag, uint16_t instance) {
  // The following priorities are used to determine the HCI port number:
  //
  // 1. The CLI flag `-hci_port`.
  // 2. The environment variable `NETSIM_HCI_PORT`.
  // 3. The default value `DEFAULT_HCI_PORT`.
  uint32_t hci_port = 0;
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

}  // namespace osutils
}  // namespace netsim
