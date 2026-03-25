# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_test_suite")
load("//next:defs.bzl", "netsim_rust_library")

# Rootcanal

# Note: This is rootcanal-rs -- the rust wrapper for lib_rootcanal_ffi
# but that name taken by a rootcanal rust component.

netsim_rust_library(
    name = "rootcanal",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "rootcanal",
    edition = "2021",
    enable_integration_test = False,
    visibility = ["//next:__subpackages__"],
    deps = [
        "@bytes",
        "@log",
        "@rootcanal//:lib_rootcanal_ffi",
        "@thiserror",
    ],
)

rust_test_suite(
    name = "tests",
    size = "small",
    srcs = glob(["tests/**/*.rs"]),
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        ":rootcanal",
        "//next/packets",
        "@bytes",
        "@cxx.rs//:cxx",
        "@env_logger",
        "@log",
        "@pdl-runtime",
        "@rootcanal//:rootcanal_rs",
        "@tokio",
    ],
)
