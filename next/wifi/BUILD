# Copyright 2025 The Android Open Source Project

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "wifi",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = [
        "//next/common",
        "//next/hostapd-rs",
        "//next/http-proxy",
        "//next/libslirp-rs",
        "//next/packets",
        "//next/proto",
        "@bytes",
        "@log",
        "@pdl-runtime",
        "@socket2",
        "@tokio",
    ],
)

rust_test(
    name = "tests",
    compile_data = [
        "src/test_packets/hwsim_cmd_frame.csv",
        "src/test_packets/hwsim_cmd_frame_mdns.csv",
        "src/test_packets/hwsim_cmd_tx_info.csv",
        "src/test_packets/hwsim_cmd_frame_request.csv",
        "src/test_packets/hwsim_cmd_tx_info_response.csv",
        "src/test_packets/hwsim_cmd_frame_request_mdns.csv",
        "src/test_packets/hwsim_cmd_tx_info_response_mdns.csv",
        "src/test_packets/hwsim_cmd_frame_response_no_transmitter_flags_tx_info.csv",
        "src/test_packets/hwsim_cmd_frame_response_no_cookie.csv",
    ],
    crate = ":wifi",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        "@wpa_supplicant_8//:hostapd_c_lib",
    ],
)
