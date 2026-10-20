# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "websocket-server",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "websocket_server",
    edition = "2024",
    test_deps = [
        "//next/actor-framework:testing",
        "//next/common",
        "@mockall",
    ],
    deps = [
        "//next/device-actor",
        "//next/device-api",
        "//next/model",
        "@anyhow",
        "@base64",
        "@bytes",
        "@futures",
        "@http",
        "@httparse",
        "@ring",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@tracing",
        "@tungstenite",
    ],
)
