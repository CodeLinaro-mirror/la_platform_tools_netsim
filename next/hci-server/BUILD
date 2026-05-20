# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "hci-server",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "hci_server",
    edition = "2024",
    deps = [
        "//next/device-actor",
        "//next/device-api",
        "//next/model",
        "@bytes",
        "@futures",
        "@tokio",
        "@tokio-stream",
        "@tokio-util",
        "@tracing",
    ],
)
