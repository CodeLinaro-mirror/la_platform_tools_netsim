# Copyright 2023-2025 The Android Open Source Project
#
# Common Bazel commands for this package:
#
# Build the binary:
#   bazel build //next/daemon
#
# Run the binary:
#   bazel run //next/daemon
#
# Run the tests:
#   bazel test //next/daemon:tests

load(
    "@rules_rust//rust:defs.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "daemon-lib",
    srcs = [
        "src/args.rs",
        "src/config.rs",
        "src/lib.rs",
        "src/logger.rs",
        "src/netsimd.rs",
        "src/platform.rs",
    ],
    crate_name = "daemon",
    edition = "2021",
    deps = [
        "//next/bluetooth",
        "//next/devices",
        "//next/netsim-api",
        "//next/packet-stream",
        "@chrono",
        "@clap",
        "@env_logger",
        "@futures",
        "@log",
        "@named-lock",
        "@tokio",
    ] + select({
        "@platforms//os:linux": ["@rustutils"],
        "//conditions:default": [],
    }),
)

rust_binary(
    name = "daemon",
    srcs = ["src/main.rs"],
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    deps = [
        ":daemon-lib",
        "@tokio",
    ],
)

rust_test(
    name = "tests",
    srcs = [
        "tests/config_test.rs",
        "tests/integration_test.rs",
        "tests/tests.rs",
    ],
    crate_root = "tests/tests.rs",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    deps = [
        ":daemon-lib",
        "//next/netsim-api",
        "//next/packet-stream",
        "@bytes",
        "@futures",
        "@rand",
        "@tokio",
    ],
)
