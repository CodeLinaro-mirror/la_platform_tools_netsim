# Copyright 2025 - The Android Open Source Project
#
# Common Bazel commands for this package:
#
# Build the library:
#   bazel build //rust/packet-stream
#
# Run all tests:
#   bazel test //rust/packet-stream:tests
#
# Run the linter (uses bazel test):
#   bazel test //rust/packet-stream:clippy

load(
    "@rules_rust//rust:defs.bzl",
    "rust_clippy",
    "rust_doc",
    "rust_library",
    "rust_test",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "packet-stream",
    srcs = glob(["src/**/*.rs"]),
    proc_macro_deps = [
        "@async-trait",
        "@serde_derive",
    ],
    deps = [
        "@bytes",
        "@futures",
        "@libc",
        "@nix",
        "@rand",
        "@rustutils",
        "@serde",
        "@serde_json",
        "@socket2",
        "@tokio",
        "@tokio-stream",
        "@tokio-util",
    ],
)

rust_test(
    name = "unit_tests",
    crate = ":packet-stream",
    tags = ["general_tests"],
    deps = [
        "@bytes",
        "@nix",
        "@rustutils",
        "@serde_json",
        "@tokio",
    ],
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    proc_macro_deps = ["@async-trait"],
    tags = ["general_tests"],
    deps = [
        ":packet-stream",
        "@bytes",
        "@futures",
        "@nix",
        "@rustutils",
        "@serde_json",
        "@tokio",
    ],
)

rust_clippy(
    name = "clippy",
    deps = [":packet-stream"],
)

rust_doc(
    name = "doc",
    crate = ":packet-stream",
)
