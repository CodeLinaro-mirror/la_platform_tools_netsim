# Copyright 2025 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Common Bazel commands for this package:
#
# Build the library:
#   bazel build //next/modem-rs-derive
#
# Run all tests:
#   bazel test //next/modem-rs-derive:tests
#
# Run the linter (uses bazel test):
#   bazel test //next/modem-rs-derive:clippy

load(
    "@rules_rust//rust:defs.bzl",
    "rust_clippy",
    "rust_doc",
    "rust_doc_test",
    "rust_proc_macro",
)

package(default_visibility = ["//visibility:public"])

rust_proc_macro(
    name = "modem-rs-derive",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "modem_rs_derive",
    crate_root = "src/lib.rs",
    edition = "2021",
    deps = [
        "@nom",
        "@proc-macro2",
        "@quote",
        "@syn",
    ],
)

rust_clippy(
    name = "clippy",
    deps = [":modem-rs-derive"],
)

rust_doc(
    name = "doc",
    crate = ":modem-rs-derive",
)

rust_doc_test(
    name = "doc_test",
    crate = ":modem-rs-derive",
    target_compatible_with = ["@platforms//os:linux"],
)
