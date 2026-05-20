# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "ethernet-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "ethernet_actor",
    edition = "2024",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/actor-framework",
        "//next/device-actor",
        "//next/model",
        "//next/slirp-actor",
        "@tokio",
        "@tracing",
    ],
)
