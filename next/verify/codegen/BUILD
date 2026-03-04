load("//next:defs.bzl", "netsim_rust_binary")

package(default_visibility = ["//visibility:public"])

netsim_rust_binary(
    name = "verify_codegen",
    srcs = [
        "src/kotlin.rs",
        "src/main.rs",
        "src/rust.rs",
        "src/utils.rs",
    ],
    edition = "2021",
    deps = [
        "@gherkin",
        "@proc-macro2",
        "@quote",
        "@regex",
        "@syn",
    ],
)
