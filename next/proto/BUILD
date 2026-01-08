load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "proto",
    srcs = glob(["src/*.rs"]),
    crate_name = "netsim_proto",
    edition = "2021",
    enable_clippy = False,
    enable_unit_test = False,
    strict_warnings = False,
    deps = [
        "@grpcio",
        "@protobuf-rust",
    ],
)
