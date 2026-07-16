# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load(
    "@rules_rust//rust:defs.bzl",
    "rustfmt_test",
)

exports_files([
    "rustfmt.toml",
    "netsimd.cc",
])

rustfmt_test(
    name = "fmt",
    tags = ["manual"],
    targets = [
        "//rust/capture:netsim_capture",
        "//rust/cli:netsim_cli",
        "//rust/common:netsim_common",
        "//rust/daemon:netsim_daemon",
        "//rust/hostapd-rs:netsim_hostapd_rs",
        "//rust/http-proxy:netsim_http_proxy",
        "//rust/libslirp-rs:netsim_libslirp_rs",
        "//rust/packets:netsim_packets",
    ],
)
