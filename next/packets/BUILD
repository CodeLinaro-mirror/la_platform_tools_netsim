# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//:__subpackages__"])

netsim_rust_library(
    name = "packets",
    srcs = glob(["src/**/*.rs"]) + [
        "//:netsim_link_layer_packets_rust_gen",
    ],
    compile_data = glob([
        "src/**/test_data/**",
    ]),
    crate_name = "netsim_packets",
    edition = "2024",
    enable_doc_test = False,
    proc_macro_deps = [
        "@num-derive",
    ],
    rustc_env = {
        "OUT_DIR": "../../..",
    },
    rustc_flags = ["--cap-lints=allow"],
    deps = [
        "@bytes",
        "@hex",
        "@num-traits",
        "@pdl-runtime",
        "@serde",
        "@serde_json",
        "@zerocopy",
    ],
)

rust_test(
    name = "packets_test",
    crate = ":packets",
    data = glob(["src/**/test_data/**"]),
    proc_macro_deps = [
        "@num-derive",
    ],
    deps = [
        "@num-traits",
        "@zerocopy",
    ],
)

rust_test(
    name = "tests",
    srcs = ["tests/integration_tests.rs"],
    edition = "2024",
    deps = [
        ":packets",
        "@num-traits",
        "@zerocopy",
    ],
)
