# Copyright 2025 - The Android Open Source Project
"""bitflags bazel build rule"""

load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "bitflags",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "bitflags",
    crate_root = "src/lib.rs",
    edition = "2021",
)
