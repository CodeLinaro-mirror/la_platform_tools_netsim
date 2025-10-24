# Copyright 2025 - The Android Open Source Project
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
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
        "@serde_derive",
    ],
    deps = [
        "@bytes",
        "@futures",
        "@libc",
        "@rand",
        "@serde",
        "@serde_json",
        "@socket2",
        "@tokio",
        "@tokio-stream",
        "@tokio-util",
    ] + select({
        "@platforms//os:linux": ["@rustutils"],
        "//conditions:default": [],
    }),
)

rust_test(
    name = "unit_tests",
    crate = ":packet-stream",
    edition = "2021",
    tags = ["general_tests"],
    deps = [
        "@bytes",
        "@serde_json",
        "@tokio",
    ] + select({
        "@platforms//os:linux": ["@rustutils"],
        "//conditions:default": [],
    }),
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = ["@async-trait"],
    tags = ["general_tests"],
    deps = [
        ":packet-stream",
        "@bytes",
        "@futures",
        "@serde_json",
        "@tokio",
    ] + select({
        "@platforms//os:linux": ["@rustutils"],
        "//conditions:default": [],
    }),
)

rust_clippy(
    name = "clippy",
    deps = [":packet-stream"],
)

rust_doc(
    name = "doc",
    crate = ":packet-stream",
)

# rust_doc_test(
#     name = "doc_test",
#     crate = ":packet-stream",
# )
