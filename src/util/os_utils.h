/*
 * Copyright 2022 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#pragma once
// OS specific utility functions.

#include <filesystem>
#include <string>

namespace netsim {
namespace osutils {

/**
 * Return the path containing runtime user files.
 */
std::filesystem::path GetDiscoveryDirectory();

/**
 * \brief Run a child process in the background
 *
 * The Daemon() function creates a child process that detaches from the
 * controlling terminal and runs in the background as a system daemon.
 *
 * This implementation follows unistd Daemon(3) with these differences:
 *
 * 1. Parent returns with pid or -1 instead of exiting.
 *
 * In our uses of daemon the parent is not long running, it exits after
 * completing the one-short command. For this reason it is not necessary to use
 * the double fork approach.
 */
int Daemon();

}  // namespace osutils
}  // namespace netsim
