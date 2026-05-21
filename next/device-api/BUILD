# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "@serde",
]

netsim_rust_library(
    name = "device-api",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "device_api",
    crate_root = "src/lib.rs",
    edition = "2024",
    deps = DEPS,
)
