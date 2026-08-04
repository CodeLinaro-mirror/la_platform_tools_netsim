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

genrule(
    name = "netsim-ui",
    srcs = ["//ui:netsim_ui_files"],
    outs = [
        "netsim-ui/index.html",
        "netsim-ui/js/device-info.js",
        "netsim-ui/js/device-list.js",
        "netsim-ui/js/navigation-bar.js",
        "netsim-ui/js/packet-info.js",
        "netsim-ui/js/license-info.js",
        "netsim-ui/js/customize-map-button.js",
        "netsim-ui/js/pyramid-sprite.js",
        "netsim-ui/js/device-dragzone.js",
        "netsim-ui/js/device-map.js",
        "netsim-ui/js/device-dropzone.js",
        "netsim-ui/js/device-observer.js",
        "netsim-ui/js/netsim-app.js",
        "netsim-ui/js/cube-sprite.js",
        "netsim-ui/dev.html",
        "netsim-ui/node_modules/tslib/tslib.es6.js",
        "netsim-ui/assets/grid-background.svg",
        "netsim-ui/assets/netsim-logo.svg",
        "netsim-ui/assets/netsim-logo-b.svg",
        "netsim-ui/assets/polar-background.svg",
        "netsim-ui/assets/hexagonal-background.png",
    ],
    cmd = """
      set -e
      mkdir -p $(@D)/netsim-ui
      # Use a sample path from the source list to find the root 'dist' directory
      source_path=$$(echo $(locations //ui:netsim_ui_files) | cut -d' ' -f1)
      dist_dir=$${source_path%/dist/*}/dist
      cp -r $${dist_dir}/. $(@D)/netsim-ui/
    """,
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
