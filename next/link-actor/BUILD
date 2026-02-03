load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/link-api",
    "//next/model",
    "//proto",
    "@anyhow",
    "@log",
    "@serde",
    "@tokio",
    "@tokio-stream",
    "@thiserror",
]

netsim_rust_library(
    name = "link-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "link_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
        "@thiserror-impl",
    ],
    test_deps = ["//next/client"],
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
        "//next/link-api:testing",
        "//next/model:testing",
        "@log",
        "@tokio",
    ],
)
