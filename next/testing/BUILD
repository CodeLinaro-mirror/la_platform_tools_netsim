# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

netsim_rust_library(
    name = "netsim-testing",
    testonly = True,
    srcs = glob(["src/**/*.rs"]),
    crate_features = ["testing"],
    crate_name = "netsim_testing",
    edition = "2024",
    enable_rustfmt = False,
    visibility = ["//next:__subpackages__"],
    deps = [
        "//next/common",
        "@bytes",
        "@futures",
        "@tokio",
        "@tokio-stream",
    ],
)
