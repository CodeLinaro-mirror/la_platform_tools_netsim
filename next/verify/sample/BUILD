load("@rules_rust//rust:defs.bzl", "rust_binary")

package(default_visibility = ["//visibility:public"])

genrule(
    name = "generate_glue",
    srcs = ["src/main.rs"],
    outs = ["glue.rs"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(location src/main.rs) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

rust_binary(
    name = "sample_host",
    testonly = True,
    srcs = ["src/main.rs"],
    compile_data = glob(["features/*.feat"]) + [":generate_glue"],
    edition = "2021",
    rustc_env = {
        "GLUE_RS": "$(location :generate_glue)",
    },
    deps = [
        "//next/verify/lib",
        "@tokio",
    ],
)
