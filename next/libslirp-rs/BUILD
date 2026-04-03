# Copyright 2025 The Android Open Source Project

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "libslirp-rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "libslirp_rs",
    crate_root = "src/lib.rs",
    edition = "2021",
    enable_integration_test = False,
    select_deps = select({
        "@bazel_tools//src/conditions:windows": ["@winapi"],
        "@platforms//os:linux": ["@libc"],
        "@platforms//os:macos": ["@libc"],
    }),
    deps = [
        "@bytes",
        "@glib//glib",
        "@libslirp",
        "@tokio",
        "@tracing",
    ],
)
