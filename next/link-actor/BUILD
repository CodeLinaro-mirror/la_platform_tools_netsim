load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/link-api",
    "//next/model",
    "//proto",
    "@anyhow",
    "@log",
    "@serde",
    "@tokio",
    "@tokio-stream",
    "@thiserror",
]

netsim_rust_library(
    name = "link-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "link_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
        "@thiserror-impl",
    ],
    deps = DEPS,
)
