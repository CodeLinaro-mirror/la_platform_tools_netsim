load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//tools/netsim:__subpackages__"])

rust_library(
    name = "cell",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/netsim-api",
        "@log",
        "@tokio",
    ],
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    deps = [
        ":cell",
        "//next/netsim-api",
        "@tokio",
    ],
)
