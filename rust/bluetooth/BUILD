# Copyright 2025 - The Android Open Source Project
#
# Common Bazel commands for this package:
#
# Build the library:
#   bazel build //rust/bluetooth
#
# Run all tests:
#   bazel test //rust/bluetooth:tests
#
# Run the linter (uses bazel test):
#   bazel test //rust/bluetooth:clippy

load(
    "@rules_rust//rust:defs.bzl",
    "rust_clippy",
    "rust_doc",
    "rust_library",
    "rust_test",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "bluetooth",
    srcs = glob(["src/**/*.rs"]),
    proc_macro_deps = [
        "@async-trait",
        "@zerocopy-derive",
    ],
    deps = [
        "//:netsim_proto",
        "//rust/netsim-api",
        "//rust/rootcanal",
        "@bytes",
        "@env_logger",
        "@hex",
        "@log",
        "@protobuf-rust",
        "@rand",
        "@thiserror",
        "@tokio",
        "@zerocopy",
    ],
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    proc_macro_deps = ["@async-trait"],
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        ":bluetooth",
        "//:netsim_proto",
        "//rust/netsim-api",
        "//rust/rootcanal",
        "@bytes",
        "@env_logger",
        "@log",
        "@rootcanal//:lib_rootcanal_ffi",
        "@tokio",
    ],
)

rust_clippy(
    name = "clippy",
    deps = [":bluetooth"],
)

rust_doc(
    name = "doc",
    crate = ":bluetooth",
)
