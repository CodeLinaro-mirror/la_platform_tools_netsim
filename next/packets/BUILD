# Copyright 2026 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//:__subpackages__"])

netsim_rust_library(
    name = "packets",
    srcs = glob(["src/**/*.rs"]),
    compile_data = glob([
        "src/**/test_data/**",
    ]),
    crate_name = "netsim_packets",
    edition = "2021",
    proc_macro_deps = [
        "@num-derive",
    ],
    deps = [
        "@hex",
        "@num-traits",
        "@serde",
        "@serde_json",
        "@zerocopy",
    ],
)

rust_test(
    name = "packets_test",
    crate = ":packets",
    data = glob(["src/**/test_data/**"]),
    proc_macro_deps = [
        "@num-derive",
    ],
    deps = [
        "@num-traits",
        "@zerocopy",
    ],
)

rust_test(
    name = "tests",
    srcs = ["tests/integration_tests.rs"],
    edition = "2021",
    deps = [
        ":packets",
        "@num-traits",
        "@zerocopy",
    ],
)
