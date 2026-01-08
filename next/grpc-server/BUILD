load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "grpc-server",
    srcs = glob(["src/*.rs"]),
    crate_name = "grpc_server",
    edition = "2021",
    enable_clippy = False,
    enable_doc_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/actor-framework",
        "//next/client",
        "//next/device-api",
        "//next/link-api",
        "//next/model",
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
