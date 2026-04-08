# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_binary")

package(default_visibility = ["//:__subpackages__"])

netsim_rust_binary(
    name = "netsim",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    tags = ["no-sandbox"],
    deps = [
        "//next/common",
        "//next/packets",
        "//proto",
        "@clap",
        "@futures",
        "@futures-util",
        "@grpcio",
        "@hex",
        "@protobuf-rust",
        "@serde",
        "@serde_json",
        "@tracing",
    ],
)

rust_test(
    name = "netsim_test",
    srcs = glob(["src/**/*.rs"]),
    crate_root = "src/main.rs",
    edition = "2021",
    deps = [
        "//next/common",
        "//next/packets",
        "//proto",
        "@clap",
        "@futures",
        "@futures-util",
        "@grpcio",
        "@hex",
        "@protobuf-rust",
        "@serde",
        "@serde_json",
        "@tracing",
    ],
)
