# Copyright 2025 - The Android Open Source Project
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
    crate_name = "modem_rs",
    edition = "2021",
    features = ["test-utils"],
    proc_macro_deps = [
        "@async-trait",
        "@serde_derive",
        "//next/modem-rs-derive",
    ],
    test_deps = [
        "@env_logger",
    ],
    deps = [
        "//next/model",
        "@anyhow",
        "@bytes",
        "@futures",
        "@hex",
        "@log",
        "@nom",
        "@serde",
        "@serde_json",
        "@tokio",
        "@zerocopy",
    ],
)
