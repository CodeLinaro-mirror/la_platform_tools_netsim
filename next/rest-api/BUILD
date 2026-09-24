# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "rest-api",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_rest_api",
    edition = "2024",
    enable_integration_test = False,
    enable_unit_test = False,
    deps = [
        "//next/model",
        "@serde",
    ],
)
