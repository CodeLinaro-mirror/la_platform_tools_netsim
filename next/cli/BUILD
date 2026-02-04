load("//next:defs.bzl", "netsim_rust_binary")

package(default_visibility = ["//:__subpackages__"])

netsim_rust_binary(
    name = "netsim",
    srcs = glob(["src/*.rs"]),
    edition = "2021",
    deps = [
        "//next/common",
        "//proto",
        "@clap",
        "@futures",
        "@futures-util",
        "@grpcio",
        "@hex",
        "@log",
        "@protobuf-rust",
        "@tokio",
    ],
)
