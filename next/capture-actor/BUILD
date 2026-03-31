# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/capture-api",
    "//next/common",
    "//next/model",
    "//next/packets",
    "@bytes",
    "@futures",
    "@serde",
    "@thiserror",
    "@tokio",
    "@tokio-stream",
    "@tracing",
    "@zerocopy",
]

netsim_rust_library(
    name = "capture-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "capture_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
