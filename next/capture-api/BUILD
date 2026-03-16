load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "@anyhow",
    "@bytes",
    "@futures",
    "@serde",
    "@tokio",
]

netsim_rust_library(
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
