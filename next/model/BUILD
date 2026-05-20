# Copyright 2023 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

NETSIM_API_DEPS = [
    "@bytes",
    "@futures",
    "//next/types",
    "@serde",
    "@thiserror",
    "@tokio",
    "@tokio-stream",
    "@tracing",
]

netsim_rust_library(
    name = "model",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_model",
    edition = "2024",
    proc_macro_deps = [
        "@async-trait",
    ],
    testing_deps = ["@mockall"],
    deps = NETSIM_API_DEPS,
)
