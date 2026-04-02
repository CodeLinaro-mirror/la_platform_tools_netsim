# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

SELECT_DEPS = select({
    "@platforms//os:linux": [
        "@nix",
        "@libc",
    ],
    "//conditions:default": [],
})

netsim_rust_library(
    name = "wifi-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "wifi_actor",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    select_deps = SELECT_DEPS,
    test_deps = [
        "//next/device-api:testing",
        "@tracing-subscriber",
    ],
    deps = [
        "//next/actor-framework",
        "//next/ap-actor",
        "//next/device-actor",
        "//next/model",
        "//next/packets",
        "//next/slirp-actor",
        "//proto",
        "@bytes",
        "@futures",
        "@socket2",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@tracing",
        "@zerocopy",
    ],
)
