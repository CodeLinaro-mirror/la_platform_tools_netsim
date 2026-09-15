# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "slirp",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "slirp",
    edition = "2024",
    select_deps = select({
        "@bazel_tools//src/conditions:windows": [
            "@windows-sys",
        ],
        "//conditions:default": [],
    }),
    test_deps = [
        "@tempfile",
    ],
    deps = [
        "//next/packets",
        "@bytes",
        "@log",
        "@rand",
        "@serde",
        "@serde_json",
        "@tokio",
        "@zerocopy",
    ],
)
