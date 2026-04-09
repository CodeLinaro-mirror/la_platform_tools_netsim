# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//:__subpackages__"])

exports_files(["bin/netsim.rs"])

rust_library(
    name = "netsim_cli",
    srcs = glob(["src/*.rs"]),
    crate_name = "netsim_cli",
    edition = "2021",
    deps = [
        "//proto",
        "//rust/common:netsim_common",
        "@clap",
        "@futures",
        "@futures-util",
        "@grpcio",
        "@hex",
        "@log",
        "@protobuf-rust",
    ],
)
