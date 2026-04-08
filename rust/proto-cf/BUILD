# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load(
    "@rules_rust//rust:defs.bzl",
    "rust_library",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "proto-cf",
    srcs = glob(["src/*.rs"]),
    crate_name = "netsim_proto",
    edition = "2021",
    deps = [
        "@grpcio",
        "@protobuf-rust",
    ],
)
