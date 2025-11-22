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
        "//next/model",
        "@bytes",
        "@futures",
        "@futures-util",
        "@libc",
        "@log",
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
    name = "unit-tests",
    crate = ":packet-stream",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
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
    srcs = [
        "tests/integration_tests.rs",
        "tests/mod.rs",
        "tests/test_connection_lifecycle.rs",
        "tests/test_dual_fd_final.rs",
        "tests/test_init_info.rs",
    ],
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = ["@async-trait"],
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = [
        "general_tests",
        "requires-network",
    ],
    deps = [
        ":packet-stream",
        ":unit-tests",
        "//next/model",
        "//next/proto",
        "@bytes",
        "@futures",
        "@grpcio",
        "@protobuf-rust",
        "@serde_json",
        "@tokio",
        "@tokio-stream",
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
