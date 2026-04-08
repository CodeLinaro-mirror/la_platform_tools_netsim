# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/device-actor",
    "//next/model",
    "@bytes",
    "@thiserror",
    "@futures",
    "@tracing",
    "@pdl-runtime",
    "@pica",
    "@tokio",
    "@serde",
    "@glam",
]

netsim_rust_library(
    name = "uwb-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = ["testing"],
    crate_name = "uwb_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    test_deps = [
        "//next/device-api:testing",
        "//next/testing",
    ],
    deps = DEPS,
)
