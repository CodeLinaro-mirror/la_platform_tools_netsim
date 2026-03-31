# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/link-api",
    "//next/model",
    "@futures",
    "@tracing",
]

netsim_rust_library(
    name = "link-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "link_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    test_deps = [
        "@tokio",
    ],
    deps = DEPS,
)
