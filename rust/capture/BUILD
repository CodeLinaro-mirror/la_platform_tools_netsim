# Copyright 2025 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "netsim_capture",
    srcs = glob(["src/**/*.rs"]),
    aliases = {
        "//rust/libslirp-rs:netsim_libslirp_rs": "libslirp_rs",
    },
    crate_name = "netsim_capture",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@zerocopy-derive",
    ],
    deps = [
        "@tokio",
        "@zerocopy",
    ],
)

rust_test(
    name = "netsim_capture_tests",
    crate = ":netsim_capture",
    tags = ["general_tests"],
)
