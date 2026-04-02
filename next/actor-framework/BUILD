# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "@bytes",
    "@futures",
    "@tokio",
    "@tokio-stream",
    "@tokio-util",
    "@thiserror",
    "@tracing",
    "//next/model",
]

netsim_rust_library(
    name = "actor-framework",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "actor_framework",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    testing_deps = ["@mockall"],
    deps = DEPS,
)
