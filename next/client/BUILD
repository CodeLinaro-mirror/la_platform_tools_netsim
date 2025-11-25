load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/model",
    "//next/device-actor",
    "@tokio",
    "@log",
]

rust_library(
    name = "client",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "client",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
