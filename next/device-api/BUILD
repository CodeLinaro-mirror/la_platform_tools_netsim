load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/model",
    "@anyhow",
    "@log",
    "@serde",
    "@tokio",
]

rust_library(
    name = "device-api",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "device_api",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
