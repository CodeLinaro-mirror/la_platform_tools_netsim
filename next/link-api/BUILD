load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "//next/proto",
    "//next/actor-framework",
    "@bytes",
    "@serde",
    "@thiserror",
]

netsim_rust_library(
    name = "link-api",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "link_api",
    crate_root = "src/lib.rs",
    edition = "2021",
    enable_clippy = False,
    enable_doc_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    strict_warnings = False,
    deps = DEPS,
)
