load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "@anyhow",
    "@bytes",
    "@futures",
    "@serde",
    "@tokio",
]

rust_library(
    name = "capture-api",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "capture_api",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
