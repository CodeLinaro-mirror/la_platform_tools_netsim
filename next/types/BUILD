load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "netsim-types",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    visibility = ["//visibility:public"],
    deps = [
        "@serde",
    ],
)
