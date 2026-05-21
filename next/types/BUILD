# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

netsim_rust_library(
    name = "types",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_types",
    edition = "2024",
    visibility = ["//visibility:public"],
    deps = [
        "@serde",
    ],
)
