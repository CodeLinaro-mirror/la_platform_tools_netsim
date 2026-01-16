load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/link-api",
    "//next/model",
    "//next/proto:proto",
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
    enable_clippy = False,
    enable_doc_test = False,
    proc_macro_deps = [
        "@async-trait",
        "@thiserror-impl",
    ],
    deps = DEPS,
)

rust_test(
    name = "integration_test",
    srcs = glob(["tests/**/*.rs"]),
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS + [
        ":link-actor",
        "//next/client",
    ],
)

rust_test(
    name = "tests",
    crate = ":link-actor",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/client",
    ],
)
