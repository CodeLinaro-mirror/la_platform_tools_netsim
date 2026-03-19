# Copyright 2025 The Android Open Source Project

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "http-proxy",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "http_proxy",
    crate_root = "src/lib.rs",
    edition = "2021",
    enable_integration_test = False,
    deps = [
        "//next/libslirp-rs",
        "@base64",
        "@bytes",
        "@etherparse",
        "@regex",
        "@tokio",
        "@tracing",
    ],
)
