load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "slirp-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "slirp_actor",
    edition = "2021",
    test_deps = [
        "@futures",
    ],
    deps = [
        "//next/actor-framework",
        "//next/libslirp-rs",
        "//next/model",
        "@bytes",
        "@log",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
    ],
)
