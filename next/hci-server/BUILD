load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "hci-server",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "hci_server",
    edition = "2021",
    deps = [
        "//next/client",
        "//next/device-api",
        "//next/model",
        "//next/packet-stream",
        "@bytes",
        "@futures",
        "@log",
        "@tokio",
        "@tokio-stream",
        "@tokio-util",
    ],
)
