# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//:__subpackages__"])

rust_library(
    name = "netsim_packets",
    srcs = glob(["src/*.rs"]) + [
        "//:netsim_ieee80211_rust_gen",
        "//:netsim_link_layer_packets_rust_gen",
        "//:netsim_llc_rust_gen",
        "//:netsim_mac80211_hwsim_rust_gen",
        "//:netsim_netlink_rust_gen",
    ],
    crate_name = "netsim_packets",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    rustc_env = {
        "OUT_DIR": "../../..",
    },
    deps = [
        "@anyhow",
        "@bytes",
        "@pdl-runtime",
    ],
)
