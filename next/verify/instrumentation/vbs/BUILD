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
        "src/main/java/com/android/verify/vbs/ServiceDiscovery.kt",
        "src/main/java/com/android/verify/vbs/TcpEchoSteps.kt",
        "src/main/java/com/android/verify/vbs/UwbSessionManager.kt",
        "src/main/java/com/android/verify/vbs/UwbSteps.kt",
        "src/main/java/com/android/verify/vbs/VbsInstrumentation.kt",
        "src/main/java/com/android/verify/vbs/WifiSteps.kt",
        ":codegen_bluetooth_adv",
        ":codegen_connectivity",
        ":codegen_lifecycle",
        ":codegen_network",
        ":codegen_service_discovery",
        ":codegen_tcp_echo",
        ":codegen_uwb",
        ":codegen_wifi",
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

genrule(
    name = "codegen_wifi",
    srcs = ["src/main/java/com/android/verify/vbs/WifiSteps.kt"],
    outs = ["src/main/java/com/android/verify/vbs/WifiStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_connectivity",
    srcs = ["src/main/java/com/android/verify/vbs/ConnectivitySteps.kt"],
    outs = ["src/main/java/com/android/verify/vbs/ConnectivityStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_network",
    srcs = ["src/main/java/com/android/verify/vbs/NetworkSteps.kt"],
    outs = ["src/main/java/com/android/verify/vbs/NetworkStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_lifecycle",
    srcs = ["src/main/java/com/android/verify/vbs/LifecycleSteps.kt"],
    outs = ["src/main/java/com/android/verify/vbs/LifecycleStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_service_discovery",
    srcs = ["src/main/java/com/android/verify/vbs/ServiceDiscovery.kt"],
    outs = ["src/main/java/com/android/verify/vbs/ServiceDiscoveryLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_tcp_echo",
    srcs = ["src/main/java/com/android/verify/vbs/TcpEchoSteps.kt"],
    outs = ["src/main/java/com/android/verify/vbs/TcpEchoStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_uwb",
    srcs = ["src/main/java/com/android/verify/vbs/UwbSteps.kt"],
    outs = ["src/main/java/com/android/verify/vbs/UwbStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_bluetooth_adv",
    srcs = ["src/main/java/com/android/verify/vbs/BluetoothAdvSteps.kt"],
    outs = ["src/main/java/com/android/verify/vbs/BluetoothAdvStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

android_binary(
    name = "vbs",
    testonly = True,
    manifest = "AndroidManifest.xml",
    tags = ["manual"],
    deps = [":vbs-lib"],
)
