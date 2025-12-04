load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "types",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_types",
    edition = "2021",
    visibility = ["//visibility:public"],
    deps = [
        "@serde",
    ],
)
