# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "@bytes",
    "@futures",
    "@serde",
    "@tokio",
]

netsim_rust_library(
    name = "capture-api",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "capture_api",
    crate_root = "src/lib.rs",
    edition = "2024",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
