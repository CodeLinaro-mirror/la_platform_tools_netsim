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
    "//next/cell-actor",
    "//next/client",
    "//next/common",
    "//next/device-actor",
    "//next/device-api",
    "//next/grpc-server",
    "//next/hci-server",
    "//next/link-actor",
    "//next/link-api",
    "//next/model",
    "//next/modem-rs",
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
    test_deps = [
        "//proto",
        "@rand",
        "@regex",
    ],
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
