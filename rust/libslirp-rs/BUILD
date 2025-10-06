# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "netsim_libslirp_rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_libslirp_rs",
    crate_root = "src/lib.rs",
    deps = [
        "@etherparse",
        "@bytes",
        "@libc",
        "@log",
        # "@winapi", // TODO: Resolve this dependency to build for Windows
        "@tokio",
        "@libslirp",
        "@glib//glib",
    ],
)

rust_test(
    name = "netsim_libslirp_rs_tests",
    crate = ":netsim_libslirp_rs",
    tags = ["general_tests"],
)

rust_test(
    name = "netsim_libslirp_rs_integration_test",
    srcs = ["tests/integration_test.rs"],
    aliases = {
        ":netsim_libslirp_rs": "libslirp_rs",
    },
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
