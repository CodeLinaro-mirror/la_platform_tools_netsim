# Copyright 2025 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "libslirp-rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "libslirp_rs",
    crate_root = "src/lib.rs",
    edition = "2024",
    enable_integration_test = True,
    # Tests monitor process-level file descriptor counts (/proc/self/fd) to verify zero
    # resource leaks on Drop of World. Running integration tests in parallel causes FD
    # count race conditions, so serial execution (--test-threads=1) is strictly required.
    integration_test_args = ["--test-threads=1"],
    select_deps = select({
        "@bazel_tools//src/conditions:windows": ["@winapi"],
        "@platforms//os:linux": ["@libc"],
        "@platforms//os:macos": ["@libc"],
    }),
    test_deps = ["@etherparse"],
    deps = [
        "@bytes",
        "@glib//glib",
        "@libslirp",
        "@tokio",
        "@tracing",
    ],
)
