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

load("@rules_rust//rust:defs.bzl", "rust_test")
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
    deps = [
        "//next/model",
        "@bytes",
        "@hex",
        "@log",
        "@nom",
        "@serde",
        "@tokio",
    ],
)

rust_test(
    name = "integration-test",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        ":testing",
        "@env_logger",
        "@hex",
        "@log",
        "@tokio",
    ],
)
