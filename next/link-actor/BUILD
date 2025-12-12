load("@rules_rust//rust:defs.bzl", "rust_clippy", "rust_library", "rust_test")

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

rust_library(
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
    deps = DEPS,
)

rust_test(
    name = "link-actor_test",
    crate = ":link-actor",
    deps = DEPS,
)

rust_clippy(
    name = "clippy",
    deps = [":link-actor"],
)
