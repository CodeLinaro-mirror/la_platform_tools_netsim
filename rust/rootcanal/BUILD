# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test_suite")

# Rootcanal

# Note: This is rootcanal-rs -- the rust wrapper for lib_rootcanal_ffi
# but that name taken by a rootcanal rust component.

rust_library(
    name = "rootcanal",
    srcs = glob(["src/**/*.rs"]),
    visibility = ["//rust:__subpackages__"],
    deps = [
        "//rust/packets:netsim_packets",
        "@cxx.rs//:cxx",
        "@log",
        "@rootcanal//:lib_rootcanal_ffi",
        "@thiserror",
    ],
)

rust_test_suite(
    name = "tests",
    size = "small",
    srcs = glob(["tests/**/*.rs"]),
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        ":rootcanal",
        "//rust/packets:netsim_packets",
        "@cxx.rs//:cxx",
        "@env_logger",
        "@log",
        "@pdl-runtime",
        "@tokio",
    ],
)
