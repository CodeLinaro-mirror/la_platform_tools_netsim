# Copyright 2025 The Android Open Source Project

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "common",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "common",
    edition = "2021",
    enable_clippy = False,
    enable_doc_test = False,
    strict_warnings = False,
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
