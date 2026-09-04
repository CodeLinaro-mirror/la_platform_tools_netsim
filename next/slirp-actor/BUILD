# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "slirp-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "slirp_actor",
    edition = "2024",
    select_deps = select({
        "@platforms//os:windows": [],
        "//conditions:default": [
            "@etherparse",
            "@socket2",
        ],
    }),
    test_deps = [],
    deps = [
        "//next/actor-framework",
        "//next/http-proxy",
        "//next/libslirp-rs",
        "//next/model",
        "//next/packets",
        "//next/slirp",
        "@bytes",
        "@futures",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@tracing",
    ],
)
