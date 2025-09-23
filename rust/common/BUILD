# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "netsim_common",
    srcs = glob(["src/**/*.rs"]),
    deps = [
        "@anyhow",
        "@chrono",
        "@env_logger",
        "@libc",
        "@log",
        "@protobuf-json-mapping",
        "@rand",
        "@zip",
    ],
)

rust_test(
    name = "tests",
    crate = ":netsim_common",
    tags = ["general_tests"],
)
