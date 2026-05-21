# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "@thiserror",
]

netsim_rust_library(
    name = "link-api",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "link_api",
    crate_root = "src/lib.rs",
    edition = "2024",
    proc_macro_deps = [
        "@async-trait",
    ],
    testing_deps = [
        "@mockall",
    ],
    deps = DEPS,
)
