# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_cc//cc:defs.bzl", "cc_library")
load("@rules_license//rules:license.bzl", "license")

package(
    default_applicable_licenses = [":license"],
)

license(
    name = "license",
    package_name = "netsim",
    copyright_notice = "Copyright 2025 - The Android Open Source Project",
    license_kinds = ["@rules_license//licenses/spdx:Apache-2.0"],
    license_text = "LICENSE",
    package_url = "https://android.googlesource.com/platform/tools/netsim/",
    visibility = ["//visibility:public"],
)

exports_files(["LICENSE"])

cc_library(
    name = "netsimd_cc_proto",
    visibility = ["//visibility:public"],
    deps = ["//proto:netsim_cc_proto"],
)

cc_library(
    name = "netsimd_cc_grpc",
    visibility = ["//visibility:public"],
    deps = ["//proto:netsim_cc_grpc"],
)

# PDL generated files
genrule(
    name = "netsim_netlink_rust_gen",
    srcs = ["pdl/netlink.pdl"],
    outs = ["netlink_packets.rs"],
    cmd = "$(location @pdl-compiler//:pdlc) --output-format rust $(SRCS) > $(OUTS)",
    tools = ["@pdl-compiler//:pdlc"],
    visibility = [
        "//next/packets:__pkg__",
        "//rust/packets:__pkg__",
    ],
)

genrule(
    name = "netsim_mac80211_hwsim_rust_gen",
    srcs = ["pdl/mac80211_hwsim.pdl"],
    outs = ["mac80211_hwsim_packets.rs"],
    cmd = "$(location @pdl-compiler//:pdlc) --output-format rust $(SRCS) > $(OUTS)",
    tools = ["@pdl-compiler//:pdlc"],
    visibility = [
        "//next/packets:__pkg__",
        "//rust/packets:__pkg__",
    ],
)

genrule(
    name = "netsim_ieee80211_rust_gen",
    srcs = ["pdl/ieee80211.pdl"],
    outs = ["ieee80211_packets.rs"],
    cmd = "$(location @pdl-compiler//:pdlc) --output-format rust $(SRCS) > $(OUTS)",
    tools = ["@pdl-compiler//:pdlc"],
    visibility = [
        "//next/packets:__pkg__",
        "//rust/packets:__pkg__",
    ],
)

genrule(
    name = "netsim_llc_rust_gen",
    srcs = ["pdl/llc.pdl"],
    outs = ["llc_packets.rs"],
    cmd = "$(location @pdl-compiler//:pdlc) --output-format rust $(SRCS) > $(OUTS)",
    tools = ["@pdl-compiler//:pdlc"],
    visibility = [
        "//next/packets:__pkg__",
        "//rust/packets:__pkg__",
    ],
)

genrule(
    name = "netsim_link_layer_packets_rust_gen",
    srcs = ["@rootcanal//:packets/link_layer_packets.pdl"],
    outs = ["link_layer_packets.rs"],
    cmd = "$(location @pdl-compiler//:pdlc) --output-format rust $(SRCS) > $(OUTS)",
    tools = ["@pdl-compiler//:pdlc"],
    visibility = [
        "//next/packets:__pkg__",
        "//rust/packets:__pkg__",
    ],
)

alias(
    name = "netsim_stripped",
    actual = "//next/cli:netsim_stripped",
)

alias(
    name = "netsim",
    actual = "//next/cli:netsim",
)

alias(
    name = "netsimd_stripped",
    actual = "//next/daemon:daemon_stripped",
)

alias(
    name = "netsimd",
    actual = "//next/daemon:daemon",
)
