# Copyright 2025 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "common",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "common",
    edition = "2021",
    test_deps = [
        "@rand",
        "@regex",
    ],
    deps = [
        "@chrono",
        "@libc",
        "@protobuf-json-mapping",
        "@tracing",
        "@tracing-log",
        "@tracing-subscriber",
        "@zip",
    ],
)
