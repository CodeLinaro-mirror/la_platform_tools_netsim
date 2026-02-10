load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/client",
    "//next/device-api",
    "//next/model",
    "@anyhow",
    "@bytes",
    "@futures",
    "@log",
    "@pica",
    "@tokio",
    "@tokio-util",
    "@tokio-stream",
    "@serde",
]

netsim_rust_library(
    name = "uwb-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "uwb_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)

rust_test(
    name = "integration-test",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        ":testing",
        "//next/actor-framework:testing",
        "//next/client:testing",
        "//next/device-api:testing",
        "//next/model:testing",
        "//next/testing",
        "@bytes",
        "@env_logger",
        "@futures",
        "@log",
        "@pdl-runtime",
        "@pica",
        "@tokio",
        "@tokio-stream",
    ],
)
