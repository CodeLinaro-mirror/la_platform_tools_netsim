# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

[genrule(
    name = "generate_%s_steps_glue" % step,
    srcs = ["src/%s_steps.rs" % step],
    outs = ["%s_steps_glue.rs" % step],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(location src/%s_steps.rs) $@" % step,
    tools = ["//next/verify/codegen:verify_codegen"],
) for step in [
    "adb",
    "android",
    "host",
    "netsim",
    "netsim_link",
]]

rust_library(
    name = "verify_host_lib",
    testonly = True,
    srcs = glob(["src/**/*.rs"]) + [
        ":generate_adb_steps_glue",
        ":generate_android_steps_glue",
        ":generate_host_steps_glue",
        ":generate_netsim_link_steps_glue",
        ":generate_netsim_steps_glue",
    ],
    compile_data = glob(["tests/features/*.feature"]) + [
        ":generate_adb_steps_glue",
        ":generate_android_steps_glue",
        ":generate_host_steps_glue",
        ":generate_netsim_link_steps_glue",
        ":generate_netsim_steps_glue",
    ],
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    rustc_env = {
        "ADB_STEPS_GLUE": "$(location :generate_adb_steps_glue)",
        "ANDROID_STEPS_GLUE": "$(location :generate_android_steps_glue)",
        "HOST_STEPS_GLUE": "$(location :generate_host_steps_glue)",
        "NETSIM_LINK_STEPS_GLUE": "$(location :generate_netsim_link_steps_glue)",
        "NETSIM_STEPS_GLUE": "$(location :generate_netsim_steps_glue)",
    },
    deps = [
        "//next/common",
        "//next/verify/lib",
        "//proto",
        "@anyhow",
        "@clap",
        "@grpcio",
        "@protobuf-rust",
        "@regex",
        "@tokio",
        "@tokio-util",
        "@tracing",
        "@tracing-subscriber",
        "@which",
    ],
)
