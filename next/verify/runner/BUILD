load("@rules_rust//rust:defs.bzl", "rust_binary")
load("@rules_shell//shell:sh_test.bzl", "sh_test")

package(default_visibility = ["//visibility:public"])

genrule(
    name = "generate_adb_steps_glue",
    srcs = ["src/adb_steps.rs"],
    outs = ["adb_steps_glue.rs"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(location src/adb_steps.rs) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "generate_android_steps_glue",
    srcs = ["src/android_steps.rs"],
    outs = ["android_steps_glue.rs"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(location src/android_steps.rs) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "generate_host_steps_glue",
    srcs = ["src/host_steps.rs"],
    outs = ["host_steps_glue.rs"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(location src/host_steps.rs) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

genrule(
    name = "generate_netsim_steps_glue",
    srcs = ["src/netsim_steps.rs"],
    outs = ["netsim_steps_glue.rs"],
    cmd = "$(location //next/verify/codegen:verify_codegen) $(location src/netsim_steps.rs) $@",
    tools = ["//next/verify/codegen:verify_codegen"],
)

rust_binary(
    name = "runner",
    testonly = True,
    srcs = glob(["src/**/*.rs"]) + [
        ":generate_adb_steps_glue",
        ":generate_android_steps_glue",
        ":generate_host_steps_glue",
        ":generate_netsim_steps_glue",
    ],
    compile_data = glob(["tests/features/*.feature"]) + [
        ":generate_adb_steps_glue",
        ":generate_android_steps_glue",
        ":generate_host_steps_glue",
        ":generate_netsim_steps_glue",
    ],
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    rustc_env = {
        "ADB_STEPS_GLUE": "$(location :generate_adb_steps_glue)",
        "ANDROID_STEPS_GLUE": "$(location :generate_android_steps_glue)",
        "HOST_STEPS_GLUE": "$(location :generate_host_steps_glue)",
        "NETSIM_STEPS_GLUE": "$(location :generate_netsim_steps_glue)",
    },
    deps = [
        "//next/verify/lib",
        "@anyhow",
        "@clap",
        "@env_logger",
        "@log",
        "@regex",
        "@tokio",
        "@tokio-util",
        "@which",
    ],
)

sh_test(
    name = "runner-e2e",
    srcs = ["runner-e2e.sh"],
    args = [
        "$(location :runner)",
        "$(location //next/verify/runner/agent:agent.apk)",
        "$(location @netsim//:netsimd)",
        "$(location //next/cli:netsim)",
    ],
    data = [
        ":runner",
        "//next/cli:netsim",
        "//next/verify/runner/agent:agent.apk",
        "@netsim//:netsimd",
    ],
    tags = ["manual"],
)
