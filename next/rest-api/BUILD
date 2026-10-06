# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "rest-api",
    srcs = glob(["src/**/*.rs"]),
    compile_data = [
        "src/openapi/reserved_idents.inc",
        "tests/openapi.yaml",
    ],
    crate_name = "netsim_rest_api",
    crate_root = "src/lib.rs",
    edition = "2024",
    proc_macro_test_deps = [
        "@async-trait",
    ],
    test_deps = [
        "//next/actor-framework",
        "@serde_yaml",
    ],
    deps = [
        "//next/ap-actor",
        "//next/device-actor",
        "//next/link-api",
        "//next/model",
        "@http-body-util",
        "@hyper",
        "@hyper-util",
        "@serde",
        "@serde_json",
        "@thiserror",
        "@tokio",
        "@tracing",
    ],
)
