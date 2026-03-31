/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <cstdint>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

namespace netsim {
namespace stringutils {

std::string ToHexString(uint8_t x, uint8_t y);
std::string ToHexString(uint8_t x);
std::string ToHexString(const uint8_t *, size_t);
std::string ToHexString(const std::vector<uint8_t> &data, int max_length);

std::string_view LTrim(const std::string_view);
std::string_view RTrim(const std::string_view);
std::string_view Trim(const std::string_view);
std::vector<std::string_view> Split(const std::string_view,
                                    const std::string_view &);
std::vector<std::string> Split(const std::string, const std::string &);
inline std::string AsString(std::string_view v) { return {v.data(), v.size()}; }

}  // namespace stringutils
}  // namespace netsim
