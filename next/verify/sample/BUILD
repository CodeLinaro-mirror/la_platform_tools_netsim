# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_binary")

package(default_visibility = ["//visibility:public"])

rust_binary(
    name = "sample_host",
    testonly = True,
    srcs = ["src/main.rs"],
    compile_data = glob(["features/*.feat"]),
    edition = "2021",
    proc_macro_deps = [
        "//next/verify/macros:verify_macros",
    ],
    deps = [
        "//next/verify/lib",
        "@anyhow",
        "@tokio",
    ],
)
