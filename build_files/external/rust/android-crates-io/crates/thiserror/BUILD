load(
    "@rules_rust//rust:defs.bzl",
    "rust_library",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "thiserror",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "thiserror",
    edition = "2021",
    proc_macro_deps = [
        "@thiserror-impl",
    ],
)
