# Copyright (C) 2025 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
qemu bazel build rule.

Scaled down qemu build rule for external/wpa_supplicant_8
"""

load("@rules_cc//cc:defs.bzl", "cc_library")

package(default_visibility = ["//visibility:public"])

cc_library(
    name = "qemu_headers",
    srcs = [
        "android/android-emu-base/android/base/files/Fd.cpp",
        "android/android-emu-base/android/base/sockets/SocketDrainer.cpp",
        "android/android-emu-base/android/base/sockets/SocketUtils.cpp",
        "android/android-emu-base/android/utils/eintr_wrapper.c",
        "android/android-emu-base/android/utils/fd.cpp",
        "android/android-emu-base/android/utils/panic.cpp",
        "android/android-emu-base/android/utils/socket_drainer.cpp",
        "android/android-emu-base/android/utils/sockets.c",
        "android/android-emu-base/android/utils/system.c",
        "android/emu/base/logging/src/android/base/logging/ColorLogSink.cpp",
        "android/emu/base/logging/src/android/base/logging/Log.cpp",
        "android/emu/base/logging/src/android/base/logging/LogConfiguration.cpp",
        "android/emu/base/logging/src/android/base/logging/LogFormatter.cpp",
        "android/emu/base/logging/src/android/base/logging/StudioLogSink.cpp",
        "android/emu/utils/src/android/utils/lock.cpp",
    ],
    hdrs = glob(["android/emu/utils/include/**/*.h"]) + [
        "android/android-emu-base/android/utils/assert.h",
        "android/android-emu-base/android/utils/compiler.h",
        "android/android-emu-base/android/utils/debug.h",
        "android/android-emu-base/android/utils/eintr_wrapper.h",
        "android/android-emu-base/android/utils/fd.h",
        "android/android-emu-base/android/utils/looper.h",
        "android/android-emu-base/android/utils/misc.h",
        "android/android-emu-base/android/utils/panic.h",
        "android/android-emu-base/android/utils/path.h",
        "android/android-emu-base/android/utils/socket_drainer.h",
        "android/android-emu-base/android/utils/sockets.h",
        "android/android-emu-base/android/utils/system.h",
        "android/emu/base/include/android/base/system/System.h",
        "android/emu/base/logging/include/android/base/logging/ColorLogSink.h",
        "android/emu/base/logging/include/android/base/logging/LogRegistry.h",
        "android/emu/base/logging/include/android/base/logging/LoggingApi.h",
        "android/emu/base/logging/include/android/base/logging/StudioLogSink.h",
        "android/emu/base/logging/include/android/base/logging/StudioMessage.h",
    ],
    includes = [
        "android/android-emu-base",
        "android/emu/base/include",
        "android/emu/base/logging/include",
        "android/emu/utils/include",
    ],
    deps = [
        "@aemu-base//:aemu-headers",
    ],
)
