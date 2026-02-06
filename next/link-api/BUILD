load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "//proto",
    "//next/actor-framework",
    "@bytes",
    "@serde",
    "@thiserror",
    "@mockall",
]

netsim_rust_library(
    name = "link-api",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "link_api",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
