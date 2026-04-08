# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

BLUETOOTH_DEPS = [
    "//next/actor-framework",
    "//next/device-actor",
    "//next/model",
    "//next/rootcanal",
    "//proto",
    "//next/packets",
    "@bytes",
    "@futures",
    "@hex",
    "@tracing",
    "@serde",
    "@thiserror",
    "@tokio",
    "@zerocopy",
]

netsim_rust_library(
    name = "bluetooth-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "bluetooth_actor",
    edition = "2021",
    enable_integration_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = BLUETOOTH_DEPS,
)

rust_test(
    name = "integration-test",
    size = "small",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = ["@async-trait"],
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        ":testing",
        "//next/actor-framework:testing",
        "//next/common",
        "//next/device-actor:testing",
        "//next/device-api:testing",
        "//next/model:testing",
        "//next/packets",
        "//next/rootcanal:testing",
        "//next/testing",
        "//proto",
        "@bytes",
        "@futures",
        "@hex",
        "@protobuf-rust",
        "@rand",
        "@rootcanal//:lib_rootcanal_ffi",
        "@serde",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@tracing",
        "@tracing-subscriber",
        "@zerocopy",
    ],
)
