load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

CELL_DEPS = [
    "//next/netsim-api",
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

rust_library(
    name = "cell",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "cell",
    edition = "2021",
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
        "@env_logger",
    ],
)
