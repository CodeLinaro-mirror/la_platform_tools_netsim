load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/capture-api",
    "//next/device-api",
    "//next/model",
    "//next/device-actor",
    "//next/capture-actor",
    "//next/link-actor",
    "//next/link-api",
    "//next/packet-stream",
    "@anyhow",
    "@bytes",
    "@futures",
    "@tokio",
    "@log",
    "@thiserror",
]

netsim_rust_library(
    name = "client",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "client",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
