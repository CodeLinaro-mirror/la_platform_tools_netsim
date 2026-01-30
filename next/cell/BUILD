load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/client",
    "//next/device-api",
    "//next/model",
    "//next/modem-rs",
    "//next/packet-stream",
    "//next/proto",
    "@tokio",
    "@log",
    "@bytes",
    "@futures",
    "@tokio-stream",
    "@thiserror",
]

netsim_rust_library(
    name = "cell",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "cell",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)
