# Copyright 2025 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "netsim_libslirp_rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_libslirp_rs",
    crate_root = "src/lib.rs",
    edition = "2021",
    deps = [
        "@bytes",
        "@etherparse",
        "@glib//glib",
        "@libc",
        "@libslirp",
        "@log",
        "@tokio",
    ] + select({
        "@bazel_tools//src/conditions:windows": ["@winapi"],
        "//conditions:default": [],
    }),
)

rust_test(
    name = "netsim_libslirp_rs_tests",
    crate = ":netsim_libslirp_rs",
    edition = "2021",
    tags = [
        "general_tests",
        "requires-network",
    ],
)

rust_test(
    name = "netsim_libslirp_rs_integration_test",
    srcs = ["tests/integration_test.rs"],
    aliases = {
        ":netsim_libslirp_rs": "libslirp_rs",
    },
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        ":netsim_libslirp_rs",
        "@bytes",
    ],
)

rust_test(
    name = "netsim_libslirp_rs_integration_udp",
    srcs = ["tests/integration_udp.rs"],
    aliases = {
        ":netsim_libslirp_rs": "libslirp_rs",
    },
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        ":netsim_libslirp_rs",
        "@bytes",
        "@etherparse",
    ],
)
