# Copyright 2025 - The Android Open Source Project
#
# Common Bazel commands for this package:
#
# Build the library:
#   bazel build //next/bluetooth
#
# Run all tests:
#   bazel test //next/bluetooth:tests
#
# Run the linter (uses bazel test):
#   bazel test //next/bluetooth:clippy

load(
    "@rules_rust//rust:defs.bzl",
    "rust_clippy",
    "rust_doc",
    "rust_doc_test",
    "rust_library",
    "rust_test",
)

package(default_visibility = ["//visibility:public"])

BLUETOOTH_DEPS = [
    "//next/model",
    "//next/rootcanal",
    "@bytes",
    "@env_logger",
    "@futures",
    "@hex",
    "@log",
    "@protobuf-rust",
    "@rand",
    "@thiserror",
    "@tokio",
    "@tokio-stream",
    "@zerocopy",
]

rust_library(
    name = "bluetooth",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
        "@zerocopy-derive",
    ],
    deps = BLUETOOTH_DEPS,
)

rust_test(
    name = "unit_tests",
    crate = ":bluetooth",
    tags = ["general_tests"],
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = ["@async-trait"],
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [":bluetooth"] + BLUETOOTH_DEPS + [
        "@rootcanal//:lib_rootcanal_ffi",
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

rust_doc_test(
    name = "doc_test",
    crate = ":bluetooth",
)
