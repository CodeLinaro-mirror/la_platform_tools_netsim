# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

# Rootcanal Legacy Control Server

netsim_rust_library(
    name = "rootcanal-server",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "rootcanal_server",
    edition = "2024",
    test_deps = [
        "//next/daemon:testing",
    ],
    visibility = ["//next:__subpackages__"],
    deps = [
        "//next/device-actor",
        "//next/model",
        "@rand",
        "@tokio",
        "@tracing",
    ],
)
