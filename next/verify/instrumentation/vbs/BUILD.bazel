# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_android//rules:rules.bzl", "android_binary")
load("@rules_kotlin//kotlin:android.bzl", "kt_android_library")

package(default_visibility = ["//visibility:public"])

kt_android_library(
    name = "vbs-lib",
    testonly = True,
    srcs = [
        "src/main/java/com/android/verify/vbs/BluetoothAdvSteps.kt",
        "src/main/java/com/android/verify/vbs/ConnectivitySteps.kt",
        "src/main/java/com/android/verify/vbs/LifecycleSteps.kt",
        "src/main/java/com/android/verify/vbs/NetworkSteps.kt",
        "src/main/java/com/android/verify/vbs/ServiceDiscoverySteps.kt",
        "src/main/java/com/android/verify/vbs/TcpEchoSteps.kt",
        "src/main/java/com/android/verify/vbs/UwbSessionManager.kt",
        "src/main/java/com/android/verify/vbs/UwbSteps.kt",
        "src/main/java/com/android/verify/vbs/VbsInstrumentation.kt",
        "src/main/java/com/android/verify/vbs/WifiSteps.kt",
    ],
    manifest = "AndroidManifest.xml",
    tags = ["manual"],
    deps = [
        "//next/verify/instrumentation/core:verify-core-lib",
        "@maven//:androidx_annotation_annotation",
        "@maven//:androidx_core_uwb_uwb",
        "@maven//:androidx_test_monitor",
        "@maven//:androidx_test_runner",
        "@maven//:org_jetbrains_kotlinx_kotlinx_coroutines_android",
        "@maven//:org_jetbrains_kotlinx_kotlinx_coroutines_core",
    ],
)

android_binary(
    name = "vbs",
    testonly = True,
    manifest = "AndroidManifest.xml",
    tags = ["manual"],
    deps = [":vbs-lib"],
)
