load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "lib",
    testonly = True,
    srcs = [
        "src/features.rs",
        "src/lib.rs",
        "src/step.rs",
        "src/utils.rs",
    ],
    crate_name = "features",
    edition = "2021",
    enable_integration_test = False,
    enable_rustfmt = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "@anyhow",
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
        "@tokio-util",
    ],
)

rust_test(
    name = "integration-test",
    srcs = [
        "tests/lib_test_world.rs",
        "tests/lib_test_world_glue.rs",
    ],
    compile_data = glob([
        "tests/features/*.feat",
        "tests/features/*.feature",
    ]),
    crate_features = ["testing"],
    crate_root = "tests/lib_test_world.rs",
    edition = "2021",
    deps = [
        ":lib",
        "//next/packets",
        "//next/testing:netsim-testing",
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
        "@zerocopy",
    ],
)

genrule(
    name = "generate_intentions_tests_glue",
    srcs = ["tests/lib_test_world.rs"],
    outs = ["tests/lib_test_world_glue.rs"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(location tests/lib_test_world.rs) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)
