load("@rules_rust//rust:defs.bzl", "rust_doc_test", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "@bytes",
    "@futures",
    "@log",
    "@tokio",
    "@tokio-stream",
    "@thiserror",
]

netsim_rust_library(
    name = "actor-framework",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "actor_framework",
    crate_root = "src/lib.rs",
    edition = "2021",
    enable_clippy = False,
    enable_doc_test = False,
    enable_unit_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    strict_warnings = False,
    deps = DEPS,
)

rust_test(
    name = "test",
    srcs = glob(["tests/**/*.rs"]),
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [":actor-framework"] + DEPS,
)

rust_doc_test(
    name = "doc_test",
    crate = ":actor-framework",
    # TODO: Enable on macOS. Disabled due to missing C++ linker wrapper in pure Rust sandbox. See tools/netsim/README.md.
    target_compatible_with = ["@platforms//os:linux"],
)
