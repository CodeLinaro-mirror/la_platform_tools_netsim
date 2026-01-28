load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

BLUETOOTH_DEPS = [
    "//next/actor-framework",
    "//next/client",
    "//next/device-api",
    "//next/model",
    "//next/rootcanal",
    "@bytes",
    "@env_logger",
    "@futures",
    "@hex",
    "@log",
    "@protobuf-rust",
    "@rand",
    "@serde",
    "@thiserror",
    "@tokio",
    "@tokio-stream",
    "@zerocopy",
]

netsim_rust_library(
    name = "bluetooth-actor",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    enable_doc_test = False,
    proc_macro_deps = [
        "@async-trait",
        "@zerocopy-derive",
    ],
    strict_warnings = False,
    deps = BLUETOOTH_DEPS,
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = ["@async-trait"],
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [":bluetooth-actor"] + BLUETOOTH_DEPS + [
        "//next/device-actor",
        "@rootcanal//:lib_rootcanal_ffi",
    ],
)
