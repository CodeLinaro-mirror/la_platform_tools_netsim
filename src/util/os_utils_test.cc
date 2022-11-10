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

#include <cstdio>
#include <filesystem>
#include <fstream>
#include <string>

#include "gtest/gtest.h"

namespace netsim {
namespace testing {
namespace {

// Test that the result of GetDiscoveryDir exists
TEST(OsUtilsTest, GetDiscoveryDir) {
  auto dir = osutils::GetDiscoveryDirectory();
  EXPECT_TRUE(std::filesystem::exists(dir));
}

// Test Daemon() by writing and reading from a shared tempfile.
TEST(OsUtilsTest, DaemonTest) {
  const char *tempFileName = std::tmpnam(NULL);
  int pid = osutils::Daemon();
  EXPECT_TRUE(pid >= 0);
  if (pid == 0) {
    // daemon process: just write pid to the temp file and exit
    std::ofstream outTempFile(tempFileName);
    outTempFile << getpid();
    outTempFile.close();
    exit(EXIT_SUCCESS);
  } else {
    // parent process: wait for the child to exit and then check that the pid
    // written to tempfile is the same we received from Daemon().
    //
    // Note: daemon is a child because we did not do the double fork.
    int wait_pid = waitpid(pid, NULL, 0);
    EXPECT_EQ(pid, wait_pid);

    std::ifstream inTempFile(tempFileName);
    int read_pid;
    inTempFile >> read_pid;
    EXPECT_EQ(read_pid, pid);
    inTempFile.close();
    EXPECT_EQ(std::remove(tempFileName), 0);
  }
}

}  // namespace
}  // namespace testing
}  // namespace netsim
