# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "netsim_hostapd_rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_hostapd_rs",
    crate_root = "src/lib.rs",
    deps = [
        "//:netsim_packets",
        "//rust/common:netsim_common",
        "@aes",
        "@anyhow",
        "@bytes",
        "@ccm",
        "@log",
        "@pdl-runtime",
        "@tokio",
    ],
)

rust_test(
    name = "netsim_hostapd_rs_tests",
    crate = ":netsim_hostapd_rs",
    tags = ["general_tests"],
)

rust_test(
    name = "netsim_hostapd_rs_integration_test",
    srcs = ["tests/integration_test.rs"],
    aliases = {
        ":netsim_hostapd_rs": "hostapd_rs",
    },
    tags = ["general_tests"],
    deps = [
        ":netsim_hostapd_rs",
        "//:netsim_packets",
        "@bytes",
        "@log",
        "@pdl-runtime",
        "@tokio",
        "@wpa_supplicant_8//:hostapd_c_lib",
    ],
)
