# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_kotlin//kotlin:android.bzl", "kt_android_library")

package(default_visibility = ["//visibility:public"])

kt_android_library(
    name = "verify-core-lib",
    testonly = True,
    srcs = [
        "src/main/java/com/android/verify/core/StepRegistry.kt",
        "src/main/java/com/android/verify/core/VerifyInstrumentation.kt",
    ],
    manifest = "AndroidManifest.xml",
    tags = ["manual"],
    deps = [
        "@maven//:androidx_annotation_annotation",
        "@maven//:androidx_test_monitor",
        "@maven//:androidx_test_runner",
    ],
)
