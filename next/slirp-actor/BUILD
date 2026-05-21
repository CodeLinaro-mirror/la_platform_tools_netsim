# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "slirp-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "slirp_actor",
    edition = "2024",
    test_deps = [
        "@futures",
    ],
    deps = [
        "//next/actor-framework",
        "//next/http-proxy",
        "//next/libslirp-rs",
        "//next/model",
        "//next/packets",
        "@bytes",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@tracing",
    ],
)
