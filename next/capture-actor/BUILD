load("@rules_rust//rust:defs.bzl", "rust_doc", "rust_doc_test", "rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/capture-api",
    "//next/model",
    "@anyhow",
    "@bytes",
    "@futures",
    "@log",
    "@serde",
    "@thiserror",
    "@tokio",
    "@zerocopy",
]

rust_library(
    name = "capture-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "capture_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = DEPS,
)

rust_doc(
    name = "doc",
    crate = ":capture-actor",
)

rust_doc_test(
    name = "doc_test",
    crate = ":capture-actor",
    # TODO: Enable on macOS. Disabled due to missing C++ linker wrapper in pure Rust sandbox. See tools/netsim/README.md.
    target_compatible_with = ["@platforms//os:linux"],
)
