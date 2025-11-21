load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "grpc-server",
    srcs = glob(["src/*.rs"]),
    crate_name = "grpc_server",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/netsim-api",
        "//next/packet-stream",
        "//next/proto",
        "@anyhow",
        "@bytes",
        "@futures",
        "@grpcio",
        "@log",
        "@protobuf",
        "@tokio",
        "@tokio-stream",
    ],
)
