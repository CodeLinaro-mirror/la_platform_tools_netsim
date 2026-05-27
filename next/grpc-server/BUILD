# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "grpc-server",
    srcs = glob(["src/*.rs"]),
    crate_name = "grpc_server",
    edition = "2024",
    proc_macro_deps = [
        "@async-trait",
    ],
    test_deps = [
        "//next/actor-framework:testing",
    ],
    deps = [
        "//next/ap-actor",
        "//next/device-actor",
        "//next/device-api",
        "//next/link-actor",
        "//next/link-api",
        "//next/model",
        "//next/packet-stream",
        "//next/packets",
        "//proto",
        "@bytes",
        "@futures",
        "@grpcio",
        "@protobuf-rust",
        "@tokio",
        "@tokio-stream",
        "@tracing",
    ],
)
