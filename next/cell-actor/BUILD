load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/client",
    "//next/device-api",
    "//next/model",
    "//next/modem-rs",
    "//next/packet-stream",
    "@zerocopy",
    "//proto",
    "@tokio",
    "@log",
    "@bytes",
    "@anyhow",
    "@futures",
    "@tokio-stream",
    "@thiserror",
]

netsim_rust_library(
    name = "cell-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "cell_actor",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)

rust_test(
    name = "integration_test",
    srcs = ["tests/integration_test.rs"],
    edition = "2021",
    deps = DEPS + [
        ":cell-actor",
        "//next/device-actor",
        "@env_logger",
    ],
)
