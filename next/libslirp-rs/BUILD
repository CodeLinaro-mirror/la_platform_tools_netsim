# Copyright 2025 The Android Open Source Project

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "libslirp-rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "libslirp_rs",
    crate_root = "src/lib.rs",
    edition = "2021",
    select_deps = select({
        "@bazel_tools//src/conditions:windows": ["@winapi"],
        "//conditions:default": [],
    }),
    deps = [
        "@bytes",
        "@etherparse",
        "@glib//glib",
        "@libc",
        "@libslirp",
        "@log",
        "@tokio",
    ],
)
