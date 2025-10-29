# Copyright 2025 - The Android Open Source Project
#
# Common Bazel commands for this package:
#
# Build the library:
#   bazel build //next/modem-rs
#
# Run all tests:
#   bazel test //next/modem-rs:tests
#
# Run the linter (uses bazel test):
#   bazel test //next/modem-rs:clippy

load(
    "@rules_rust//rust:defs.bzl",
    "rust_doc",
    "rust_library",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "modem-rs",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    features = ["test-utils"],
    proc_macro_deps = [
        "@async-trait",
        "@serde_derive",
        "//next/modem-rs-derive",
    ],
    deps = [
        "//next/netsim-api",
        "@bytes",
        "@tokio",
        "@serde",
        # "@toml",
        "@nom",
        "@hex",
        "@log",
    ],
)

# rust_test(
#     name = "unit_tests",
#     crate = ":modem-rs",
#     tags = ["general_tests"],
#     proc_macro_deps = [
#         "@async-trait",
#         "@serde_derive",
#         "//next/modem-rs-derive",

#     ],
#     deps = [
#         ":modem-rs",
#     ],
# )

# rust_test(
#     name = "tests",
#     srcs = glob(["tests/**/*.rs"]),
#     crate_root = "tests/mod.rs",
#     proc_macro_deps = ["@async-trait"],
#     tags = ["general_tests"],
#     deps = [
#         ":modem-rs",
# "@hex",
#     ],
# )

# rust_clippy(
#     name = "clippy",
#     deps = [":modem-rs"],
# )

rust_doc(
    name = "doc",
    crate = ":modem-rs",
)

# rust_doc_test(
#     name = "doc_test",
#     crate = ":modem-rs",
# )
