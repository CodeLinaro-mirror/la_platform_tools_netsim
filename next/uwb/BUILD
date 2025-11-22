load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

UWB_DEPS = [
    "@bytes",
    "@futures",
    "@log",
    "@tokio-stream",
    "@tokio",
    "//next/model",
]

rust_library(
    name = "uwb",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "uwb",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = UWB_DEPS,
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = UWB_DEPS + [
        ":uwb",
        "@env_logger",
    ],
)
