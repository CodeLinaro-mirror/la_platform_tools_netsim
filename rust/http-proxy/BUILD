# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "netsim_http_proxy",
    srcs = glob(["src/**/*.rs"]),
    aliases = {
        "//rust/libslirp-rs:netsim_libslirp_rs": "libslirp_rs",
    },
    crate_name = "netsim_http_proxy",
    crate_root = "src/lib.rs",
    deps = [
        "//rust/libslirp-rs:netsim_libslirp_rs",
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
    name = "netsim_http_proxy_tests",
    crate = ":netsim_http_proxy",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
)
