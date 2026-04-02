// Copyright 2022 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#include "util/ini_file.h"

#include <fstream>
#include <iostream>
#include <string>
#include <string_view>

#include "util/log.h"
#include "util/string_utils.h"

namespace netsim {

bool IniFile::Read() {
  data.clear();

  if (filepath.empty()) {
    BtsLogWarn("Read called without a backing ini file!");
    return false;
  }

  std::ifstream inFile(filepath);

  if (!inFile) {
    BtsLogWarn("Failed to process .ini file %s for reading.", filepath.c_str());
    return false;
  }
  std::string line;
  while (std::getline(inFile, line)) {
    auto argv = stringutils::Split(line, "=");

    if (argv.size() != 2) continue;
    auto key = stringutils::Trim(argv[0]);
    auto val = stringutils::Trim(argv[1]);
    data.emplace(key, val);
  }
  return true;
}

bool IniFile::Write() {
  if (filepath.empty()) {
    BtsLogWarn("Write called without a backing ini file!");
    return false;
  }

  std::ofstream outFile(filepath);

  if (!outFile) {
    BtsLogWarn("Failed to open .ini file %s for writing.", filepath.c_str());
    return false;
  }

  for (const auto &pair : data) {
    outFile << pair.first << "=" << pair.second << std::endl;
  }
  return true;
}

bool IniFile::HasKey(const std::string &key) const {
  return data.find(key) != std::end(data);
}

std::optional<std::string> IniFile::Get(const std::string &key) const {
  auto citer = data.find(key);
  return (citer == std::end(data)) ? std::nullopt
                                   : std::optional(citer->second);
}

void IniFile::Set(const std::string &key, std::string_view value) {
  data[key] = std::string(value);
}

}  // namespace netsim
