# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_android//rules:rules.bzl", "android_binary")
load("@rules_kotlin//kotlin:android.bzl", "kt_android_library")

package(default_visibility = ["//visibility:public"])

kt_android_library(
    name = "agent-lib",
    testonly = True,
    srcs = [
        "src/main/java/com/android/netsim/agent/BluetoothAdvSteps.kt",
        "src/main/java/com/android/netsim/agent/ConnectivitySteps.kt",
        "src/main/java/com/android/netsim/agent/LifecycleSteps.kt",
        "src/main/java/com/android/netsim/agent/NTestInstrumentation.kt",
        "src/main/java/com/android/netsim/agent/NetworkSteps.kt",
        "src/main/java/com/android/netsim/agent/ServiceDiscovery.kt",
        "src/main/java/com/android/netsim/agent/StepRegistry.kt",
        "src/main/java/com/android/netsim/agent/TcpEchoSteps.kt",
        "src/main/java/com/android/netsim/agent/UwbSessionManager.kt",
        "src/main/java/com/android/netsim/agent/UwbSteps.kt",
        "src/main/java/com/android/netsim/agent/WifiSteps.kt",
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
    srcs = ["src/main/java/com/android/netsim/agent/WifiSteps.kt"],
    outs = ["src/main/java/com/android/netsim/agent/WifiStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_connectivity",
    srcs = ["src/main/java/com/android/netsim/agent/ConnectivitySteps.kt"],
    outs = ["src/main/java/com/android/netsim/agent/ConnectivityStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_network",
    srcs = ["src/main/java/com/android/netsim/agent/NetworkSteps.kt"],
    outs = ["src/main/java/com/android/netsim/agent/NetworkStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_lifecycle",
    srcs = ["src/main/java/com/android/netsim/agent/LifecycleSteps.kt"],
    outs = ["src/main/java/com/android/netsim/agent/LifecycleStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_service_discovery",
    srcs = ["src/main/java/com/android/netsim/agent/ServiceDiscovery.kt"],
    outs = ["src/main/java/com/android/netsim/agent/ServiceDiscoveryLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_tcp_echo",
    srcs = ["src/main/java/com/android/netsim/agent/TcpEchoSteps.kt"],
    outs = ["src/main/java/com/android/netsim/agent/TcpEchoStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_uwb",
    srcs = ["src/main/java/com/android/netsim/agent/UwbSteps.kt"],
    outs = ["src/main/java/com/android/netsim/agent/UwbStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "codegen_bluetooth_adv",
    srcs = ["src/main/java/com/android/netsim/agent/BluetoothAdvSteps.kt"],
    outs = ["src/main/java/com/android/netsim/agent/BluetoothAdvStepsLoader.kt"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(SRCS) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

android_binary(
    name = "agent",
    testonly = True,
    manifest = "AndroidManifest.xml",
    tags = ["manual"],
    deps = [":agent-lib"],
)
