load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

CELL_DEPS = [
    "//next/client",
    "//next/device-api",
    "//next/model",
    "//next/modem-rs",
    "//next/packet-stream",
    "//next/proto",
    "@tokio",
    "@log",
    "@bytes",
    "@futures",
    "@tokio-stream",
    "@thiserror",
]

netsim_rust_library(
    name = "cell",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "cell",
    edition = "2021",
    enable_clippy = False,
    enable_doc_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = CELL_DEPS,
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/integration_test.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = CELL_DEPS + [
        ":cell",
        "//next/actor-framework",
        "//next/device-actor",
        "@env_logger",
    ],
)
