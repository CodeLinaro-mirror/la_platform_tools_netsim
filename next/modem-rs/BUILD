# Copyright 2025 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Common Bazel commands for this package:
#
# Build the library:
#   bazel build //next/modem-rs
#
# Run all tests:
#   bazel test //next/modem-rs:tests
#
# Run the linter (uses bazel test):
#   bazel test //next/modem-rs:clippy

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "modem-rs",
    srcs = glob(["src/**/*.rs"]),
    compile_data = [
        "src/profiles/iccprofile_for_sim0.xml",
        "src/profiles/iccprofile_for_sim0_for_CtsCarrierApiTestCases.xml",
    ],
    crate_name = "modem_rs",
    edition = "2024",
    features = ["test-utils"],
    proc_macro_deps = [
        "//next/modem-rs-derive",
    ],
    test_deps = [
        "@tempfile",
        "@tracing-subscriber",
    ],
    deps = [
        "//next/model",
        "@bytes",
        "@chrono",
        "@hex",
        "@nom",
        "@serde",
        "@serde-xml-rs",
        "@serde_json",
        "@tokio",
        "@tracing",
    ],
)
