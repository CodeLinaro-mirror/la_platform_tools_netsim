# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "hostapd-rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "hostapd_rs",
    crate_root = "src/lib.rs",
    edition = "2021",
    deps = [
        "//next/common",
        "//next/packets",
        "@aes",
        "@anyhow",
        "@bytes",
        "@ccm",
        "@log",
        "@pdl-runtime",
        "@tokio",
        "@wpa_supplicant_8//:hostapd_c_lib",
    ],
)

rust_test(
    name = "hostapd_rs_tests",
    crate = ":hostapd-rs",
    edition = "2021",
    tags = ["general_tests"],
)

rust_test(
    name = "test",
    srcs = ["tests/integration_test.rs"],
    edition = "2021",
    tags = ["general_tests"],
    deps = [
        ":hostapd-rs",
        "//next/packets",
        "@anyhow",
        "@bytes",
        "@log",
        "@pdl-runtime",
        "@tokio",
        "@zerocopy",
    ],
)
