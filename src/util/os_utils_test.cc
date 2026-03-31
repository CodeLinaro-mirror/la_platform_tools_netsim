// Copyright 2022 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#include "util/os_utils.h"

#include <cstdio>
#include <fstream>
#include <string>

#include "gtest/gtest.h"
#include "util/filesystem.h"

namespace netsim {
namespace testing {
namespace {

// Test that the result of GetDiscoveryDir exists
TEST(OsUtilsTest, GetDiscoveryDir) {
  auto dir = osutils::GetDiscoveryDirectory();
  EXPECT_TRUE(netsim::filesystem::exists(dir));
}

}  // namespace
}  // namespace testing
}  // namespace netsim
