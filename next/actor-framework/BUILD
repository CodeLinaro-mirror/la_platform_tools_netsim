load("@rules_rust//rust:defs.bzl", "rust_doc", "rust_doc_test", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "@bytes",
    "@futures",
    "@log",
    "@tokio",
    "@tokio-stream",
    "@thiserror",
]

rust_library(
    name = "actor-framework",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "actor_framework",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
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

rust_doc(
    name = "doc",
    crate = ":actor-framework",
)

rust_doc_test(
    name = "doc_test",
    crate = ":actor-framework",
    # TODO: Enable on macOS. Disabled due to missing C++ linker wrapper in pure Rust sandbox. See tools/netsim/README.md.
    target_compatible_with = ["@platforms//os:linux"],
)
