load(
    "@rules_rust//rust:defs.bzl",
    "rust_proc_macro",
)

package(default_visibility = ["//visibility:public"])

rust_proc_macro(
    name = "thiserror-impl",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "thiserror_impl",
    edition = "2021",
    deps = [
        "@proc-macro2",
        "@quote",
        "@syn",
    ],
)
