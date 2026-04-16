load("@rules_cc//cc:defs.bzl", "cc_binary", "cc_library")
load("@rules_license//rules:license.bzl", "license")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test")

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

rust_binary(
    name = "netsim",
    srcs = ["//rust/cli:bin/netsim.rs"],
    crate_name = "netsim_cli",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    deps = [
        "//rust/cli:netsim_cli",
    ],
)

rust_test(
    name = "netsim_cli_tests",
    crate = "//rust/cli:netsim_cli",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
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
    name = "netsim_daemon_cc",
    srcs = ["//rust/daemon:src/ffi.rs"],
    outs = ["netsim-daemon/src/ffi.rs.cc"],
    cmd = "$(location @cxx.rs//:codegen) $(SRCS) --cfg feature=\\\"local_ssl\\\" >> $(OUTS)",
    tools = ["@cxx.rs//:codegen"],
)

genrule(
    name = "netsim_daemon_h",
    srcs = ["//rust/daemon:src/ffi.rs"],
    outs = ["netsim-daemon/src/ffi.rs.h"],
    cmd = "$(location @cxx.rs//:codegen) $(SRCS) --cfg feature=\\\"local_ssl\\\" --header >> $(OUTS)",
    tools = ["@cxx.rs//:codegen"],
)

cc_binary(
    name = "netsimd",
    srcs = [
        "src/backend/grpc_client.h",
        "src/hci/bluetooth_facade.cc",
        "src/hci/bluetooth_facade.h",
        "src/hci/hci_packet_hub.h",
        "src/hci/hci_packet_transport.cc",
        "src/hci/hci_packet_transport.h",
        "src/hci/rust_device.cc",
        "src/hci/rust_device.h",
        "src/util/crash_report.cc",
        "src/util/crash_report.h",
        "src/util/filesystem.h",
        "src/util/ini_file.h",
        "src/util/log.cc",
        "src/util/log.h",
        ":netsim_daemon_cc",
        ":netsim_daemon_h",
        "//rust:cxx-bridge-header",
        "//rust:netsimd.cc",
    ] + select({
        "@platforms//os:windows": ["src/hci/async_manager.cc"],
        "//conditions:default": [],
    }),
    defines = ["NETSIM_ANDROID_EMULATOR"],
    includes = ["src"],
    linkopts = select({
        "@platforms//os:windows": ["ntdll.lib"],
        "//conditions:default": [],
    }),
    visibility = ["//visibility:public"],
    deps = [
        ":netsimd_cc_grpc",
        ":netsimd_cc_proto",
        "@aemu//base:aemu-base",
        "@aemu//base:aemu-base-socket-utils",
        "@c-ares//:ares",
        "@glib//glib",
        "@rootcanal//:libbt-rootcanal",
        "@wpa_supplicant_8//:hostapd_c_lib",
    ] + select({
        "@platforms//os:windows": [
            "//rust/daemon:netsim_daemon_windows",
        ],
        "//conditions:default": [
            "//rust/daemon:netsim_daemon",
        ],
    }),
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
