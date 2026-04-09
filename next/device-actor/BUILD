# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "device-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "device_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    test_deps = [
        "@mockall",
        "@serde_json",
        "@tokio-stream",
    ],
    deps = [
        "//next/actor-framework",
        "//next/capture-api",
        "//next/common",
        "//next/device-api",
        "//next/link-api",
        "//next/model",
        "//proto",
        "@bytes",
        "@futures",
        "@protobuf-json-mapping",
        "@serde",
        "@thiserror",
        "@tokio",
        "@tracing",
    ],
)
