load("@rules_rust//rust:defs.bzl", "rust_clippy", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/model",
    "//next/proto:proto",
    "@bytes",
    "@serde",
    "@thiserror",
]

rust_library(
    name = "link-api",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "link_api",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [],
    deps = DEPS,
)

rust_test(
    name = "link-api_test",
    crate = ":link-api",
    deps = DEPS,
)

rust_clippy(
    name = "clippy",
    deps = [":link-api"],
)
