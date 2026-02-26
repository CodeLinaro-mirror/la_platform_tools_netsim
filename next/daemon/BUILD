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
    "rust_test",
)
load(
    "//next:defs.bzl",
    "netsim_rust_binary",
    "netsim_rust_library",
)

package(default_visibility = ["//visibility:public"])

DAEMON_SRCS = [
    "src/args.rs",
    "src/avd_config.rs",
    "src/ini_file.rs",
    "src/lib.rs",
    "src/logger.rs",
    "src/netsimd.rs",
    "src/platform.rs",
    "src/version.rs",
]

DAEMON_DEPS = [
    "//next/actor-framework",
    "//next/ap-actor",
    "//next/bluetooth-actor",
    "//next/capture-actor",
    "//next/capture-api",
    "//next/cell",
    "//next/client",
    "//next/common",
    "//next/device-actor",
    "//next/device-api",
    "//next/grpc-server",
    "//next/hci-server",
    "//next/link-actor",
    "//next/link-api",
    "//next/model",
    "//next/packet-stream",
    "//next/slirp-actor",
    "//next/uwb-actor",
    "//next/wifi-actor",
    "@bytes",
    "@chrono",
    "@clap",
    "@env_logger",
    "@futures",
    "@grpcio",
    "@log",
    "@tempfile",
    "@tokio",
    "@tokio-stream",
    "@tracing",
    "@tracing-log",
    "@tracing-subscriber",
]

netsim_rust_library(
    name = "daemon-lib",
    srcs = DAEMON_SRCS,
    crate_name = "daemon",
    edition = "2021",
    proc_macro_deps = ["@async-trait"],
    select_deps = select({
        "@platforms//os:linux": ["@rustutils"],
        "//conditions:default": [],
    }),
    deps = DAEMON_DEPS,
)

netsim_rust_binary(
    name = "daemon",
    srcs = ["src/main.rs"],
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
        "-Cforce-frame-pointers=yes",
    ],
    deps = [
        ":daemon-lib",
        "@rand",
        "@tokio",
    ],
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/tests.rs",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["requires-network"],
    deps = [
        ":testing",
        "//next/client",
        "//next/device-api",
        "//next/link-api",
        "//next/model",
        "//next/packet-stream",
        "//proto",
        "@bytes",
        "@env_logger",
        "@futures",
        "@grpcio",
        "@log",
        "@rand",
        "@regex",
        "@tokio",
        "@tracing",
        "@tracing-log",
        "@tracing-subscriber",
    ],
)
