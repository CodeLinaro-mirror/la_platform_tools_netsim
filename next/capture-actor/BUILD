load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/model",
    "@anyhow",
    "@bytes",
    "@futures",
    "@log",
    "@serde",
    "@thiserror",
    "@tokio",
    "@zerocopy",
]

rust_library(
    name = "capture-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "capture_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
