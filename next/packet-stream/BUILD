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

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/types",
    "@bytes",
    "@futures",
    "@serde",
    "@serde_json",
    "@thiserror",
    "@tokio",
    "@tokio-util",
]

SELECT_DEPS = select({
    "@bazel_tools//src/conditions:windows": ["@tracing"],
    "//conditions:default": [],
})

netsim_rust_library(
    name = "packet-stream",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "packet_stream",
    edition = "2021",
    enable_integration_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    select_deps = SELECT_DEPS,
    deps = DEPS,
)
