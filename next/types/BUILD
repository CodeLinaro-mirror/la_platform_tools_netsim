load("//next:defs.bzl", "netsim_rust_library")

netsim_rust_library(
    name = "types",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_types",
    edition = "2021",
    visibility = ["//visibility:public"],
    deps = [
        "@serde",
    ],
)
