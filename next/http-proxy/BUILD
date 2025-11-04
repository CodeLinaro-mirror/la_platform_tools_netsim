# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "http-proxy",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "http_proxy",
    crate_root = "src/lib.rs",
    edition = "2021",
    deps = [
        "//next/libslirp-rs",
        "@base64",
        "@bytes",
        "@etherparse",
        "@httparse",
        "@log",
        "@regex",
        "@tokio",
    ],
)

rust_test(
    name = "http_proxy_tests",
    crate = ":http-proxy",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
)
