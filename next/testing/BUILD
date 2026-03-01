load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

netsim_rust_library(
    name = "netsim-testing",
    testonly = True,
    srcs = glob(["src/**/*.rs"]) + [":generate_cuke_tests_glue"],
    crate_features = ["testing"],
    crate_name = "netsim_testing",
    edition = "2021",
    enable_rustfmt = False,
    visibility = ["//next:__subpackages__"],
    deps = [
        "@bytes",
        "@env_logger",
        "@futures",
        "@gherkin",
        "@log",
        "@regex",
        "@serde",
        "@serde_json",
        "@tokio",
        "@tokio-stream",
    ],
)

rust_test(
    name = "integration-test",
    srcs = ["tests/cuke_world.rs"] + [":generate_cuke_tests_glue"],
    compile_data = glob(["tests/features/*"]),
    crate_features = ["testing"],
    crate_root = "tests/cuke_world.rs",
    edition = "2021",
    deps = [
        ":netsim-testing",
        "//next/packets",
        "@bytes",
        "@env_logger",
        "@futures",
        "@gherkin",
        "@log",
        "@regex",
        "@serde_json",
        "@tokio",
        "@tokio-stream",
        "@zerocopy",
    ],
)

rust_binary(
    name = "cuke_codegen",
    srcs = ["src/cuke_codegen.rs"],
    edition = "2021",
    deps = [
        "@proc-macro2",
        "@quote",
        "@regex",
        "@syn",
    ],
)

genrule(
    name = "generate_cuke_tests_glue",
    srcs = ["tests/cuke_world.rs"],
    outs = ["tests/cuke_world_glue.rs"],
    cmd = "$(location :cuke_codegen) $(location tests/cuke_world.rs) $@",
    tools = [":cuke_codegen"],
)
