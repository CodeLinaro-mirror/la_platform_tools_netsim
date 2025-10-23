# Copyright 2023-2025 The Android Open Source Project
#
# Common Bazel commands for this package:
#
# Build the binary:
#   bazel build //next/netsim-next
#
# Run the binary:
#   bazel run //next/netsim-next
#
# Run the tests:
#   bazel test //next/netsim-next:tests

load(
    "@rules_rust//rust:defs.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "netsim-next-lib",
    srcs = [
        "src/config.rs",
        "src/lib.rs",
        "src/logger.rs",
        "src/netsimd.rs",
        "src/platform.rs",
    ],
    crate_name = "netsim_next",
    edition = "2021",
    deps = [
        "//next/bluetooth",
        "//next/devices",
        "//next/netsim-api",
        "@chrono",
        "@env_logger",
        "@log",
        "@tokio",
    ] + select({
        "@platforms//os:linux": ["@rustutils"],
        "//conditions:default": [],
    }),
)

rust_binary(
    name = "netsim-next",
    srcs = ["src/main.rs"],
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    deps = [
        ":netsim-next-lib",
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
        ":netsim-next-lib",
        "@rand",
    ],
)
