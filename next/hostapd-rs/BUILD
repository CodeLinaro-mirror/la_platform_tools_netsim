# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "hostapd-rs",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "hostapd_rs",
    crate_root = "src/lib.rs",
    edition = "2021",
    # Doc tests fail to link against hostapd_c_lib symbols
    enable_doc_test = False,
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
    name = "integration-test",
    srcs = ["tests/integration_test.rs"],
    edition = "2021",
    tags = ["general_tests"],
    deps = [
        ":testing",
        "//next/packets",
        "@anyhow",
        "@bytes",
        "@log",
        "@pdl-runtime",
        "@tokio",
        "@zerocopy",
    ],
)
