# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_binary")

package(default_visibility = ["//visibility:public"])

rust_binary(
    name = "verify_codegen",
    srcs = [
        "src/main.rs",
        "src/rust.rs",
        "src/utils.rs",
    ],
    edition = "2021",
    deps = [
        "@proc-macro2",
        "@quote",
        "@regex",
        "@syn",
    ],
)
