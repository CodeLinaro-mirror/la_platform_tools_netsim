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
    "@anyhow",
    "@bytes",
    "@futures",
    "//next/packet-stream",
    "@tokio",
    "@log",
    "@thiserror",
]

netsim_rust_library(
    name = "client",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "client",
    crate_root = "src/lib.rs",
    edition = "2021",
    enable_clippy = False,
    enable_doc_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
