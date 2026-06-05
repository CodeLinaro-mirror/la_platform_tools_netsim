# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/device-actor",
    "//next/device-api",
    "//next/model",
    "@tracing",
    "@bytes",
    "@futures",
    "@thiserror",
]

netsim_rust_library(
    name = "nfc-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "nfc_actor",
    edition = "2024",
    proc_macro_deps = [
        "@async-trait",
    ],
    test_deps = [
        "@tracing-subscriber",
    ],
    deps = DEPS,
)
