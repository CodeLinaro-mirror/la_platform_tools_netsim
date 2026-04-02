// Copyright 2022 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#include "util/string_utils.h"

#include <cstdint>
#include <string>

#include "gtest/gtest.h"

namespace netsim {
namespace stringutils {
namespace testing {
namespace {

TEST(StringUtilsTest, ToHexTest) {
  EXPECT_EQ(ToHexString(0x12), "0x12");
  EXPECT_EQ(ToHexString(0xBE, 0xEF), "0xBEEF");
  const std::vector<uint8_t> a = {0xDE, 0xAD, 0xBE, 0xEF};
  EXPECT_EQ(ToHexString(a, 8), "DE AD BE EF");
  EXPECT_EQ(ToHexString(a, 3), "DE AD BE");
}

TEST(StringUtilsTest, TrimTest) {
  std::string s = "\n\tHello World  \r\n";
  std::string_view trimmed = Trim(s);
  EXPECT_EQ(trimmed, "Hello World");
}

TEST(StringUtilsTest, SplitTest) {
  std::string s = "a=b=c=d==";
  auto r = Split(s, "=");
  EXPECT_EQ(r.size(), 4);
  EXPECT_EQ(r[0], "a");
}

TEST(StringUtilsTest, AsStringTest) {
  std::string str = "test-string";
  std::string_view sv = str;
  EXPECT_EQ(AsString(sv), str);
}

}  // namespace
}  // namespace testing
}  // namespace stringutils
}  // namespace netsim
