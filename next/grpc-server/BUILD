load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "grpc-server",
    srcs = glob(["src/*.rs"]),
    crate_name = "grpc_server",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/device-actor",
        "//next/device-api",
        "//next/link-actor",
        "//next/link-api",
        "//next/model",
        "//next/packet-stream",
        "//proto",
        "@anyhow",
        "@bytes",
        "@futures",
        "@grpcio",
        "@log",
        "@protobuf-rust",
        "@tokio",
        "@tokio-stream",
    ],
)
