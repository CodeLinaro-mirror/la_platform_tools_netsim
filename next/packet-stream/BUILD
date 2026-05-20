# Copyright 2025 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Common Bazel commands for this package:
#
# Build the library:
#   bazel build //next/packet-stream
#
# Run all tests:
#   bazel test //next/packet-stream:tests
#
# Run the linter (uses bazel test):
#   bazel test //next/packet-stream:clippy

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/types",
    "@bytes",
    "@futures",
    "@serde",
    "@serde_json",
    "@thiserror",
    "@tokio",
    "@tokio-util",
    "@tracing",
]

netsim_rust_library(
    name = "packet-stream",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "packet_stream",
    edition = "2024",
    enable_integration_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
