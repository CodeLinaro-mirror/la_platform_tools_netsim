/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <sys/stat.h>

#include <fstream>
#include <string>

namespace netsim {
namespace filesystem {

#ifdef _WIN32
static const std::string slash = "\\";
#else
static const std::string slash = "/";
#endif

/**
 * Return if a path exists.
 */
inline bool exists(const std::string &name) {
  struct stat stat_buffer;
  return stat(name.c_str(), &stat_buffer) == 0;
}

/**
 * Return if a file exists.
 */
inline bool is_regular_file(const std::string &name) {
  // NOTE: Use fstream instead because 'S_ISREG'is undeclared identifier in
  // windows.
  std::ifstream f(name.c_str());
  return f.good();
}

}  // namespace filesystem
}  // namespace netsim
