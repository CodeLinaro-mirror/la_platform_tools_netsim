load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/device-actor",
    "//next/device-api",
    "//next/model",
    "//next/modem-rs",
    "@tokio",
    "@tracing",
    "@bytes",
    "@futures",
    "@tokio-stream",
    "@thiserror",
]

netsim_rust_library(
    name = "cell-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "cell_actor",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    test_deps = [
        "@tracing-subscriber",
    ],
    deps = DEPS,
)
