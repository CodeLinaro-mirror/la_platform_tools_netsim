/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once
// OS specific utility functions.

#include <memory>
#include <optional>
#include <string>

namespace netsim {
namespace osutils {

/**
 * Return the path containing runtime user files.
 */
std::string GetDiscoveryDirectory();

/**
 * Return the path of netsim ini file.
 */
std::string GetNetsimIniFilepath(uint16_t instance_num);

/**
 * Return the frontend grpc port.
 */
std::optional<std::string> GetServerAddress(uint16_t instance_num = 1);

}  // namespace osutils
}  // namespace netsim
