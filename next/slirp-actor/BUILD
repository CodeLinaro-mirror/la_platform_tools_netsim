load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "slirp-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "slirp_actor",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/actor-framework",
        "//next/libslirp-rs",
        "//next/model",
        "//next/packets",
        "//next/proto",
        "@anyhow",
        "@bytes",
        "@futures",
        "@log",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
    ],
)

rust_test(
    name = "integration_test",
    srcs = glob(["tests/**/*.rs"]),
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        ":slirp-actor",
        "//next/actor-framework",
        "//next/libslirp-rs",
        "@bytes",
        "@futures",
        "@log",
        "@tokio",
        "@tokio-stream",
    ],
)
