/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>

namespace netsim {

// A simple class to process init file. Modified from
// external/qemu/android/android-emu-base/android/base/files/IniFile.h
class IniFile {
 public:
  // Note that the constructor _does not_ read data from the backing file.
  // Call |Read| to read the data.
  explicit IniFile(std::string filepath = "") : filepath(std::move(filepath)) {}

  // Reads data into IniFile from the backing file, overwriting any
  // existing data.
  bool Read();

  // Writes the current IniFile to the backing file.
  bool Write();

  // Checks if a certain key exists in the file.
  bool HasKey(const std::string &key) const;

  // Gets value.
  std::optional<std::string> Get(const std::string &key) const;

  // Sets value.
  void Set(const std::string &key, std::string_view value);

 private:
  std::unordered_map<std::string, std::string> data;
  std::string filepath;
};

}  // namespace netsim
