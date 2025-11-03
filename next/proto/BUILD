load(
    "@rules_rust//rust:defs.bzl",
    "rust_library",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "proto",
    srcs = glob(["src/*.rs"]),
    crate_name = "netsim_proto",
    edition = "2021",
    deps = [
        "@protobuf-rust",
    ],
)
