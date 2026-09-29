# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "slirp-host",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "slirp_host",
    edition = "2024",
    # The integration tests bind loopback TCP listeners and send real ICMP
    # echo requests to 127.0.0.1, which the sandbox's network namespace blocks.
    tags = ["no-sandbox"],
    test_deps = ["@zerocopy"],
    deps = [
        "//next/packets",
        "//next/slirp",
        "@bytes",
        "@log",
        "@parking_lot",
        "@socket2",
        "@tokio",
    ],
)
