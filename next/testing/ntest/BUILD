load("@rules_rust//rust:defs.bzl", "rust_binary")
load("@rules_shell//shell:sh_test.bzl", "sh_test")

package(default_visibility = ["//visibility:public"])

rust_binary(
    name = "ntest",
    testonly = True,
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    # Restrict to Linux only
    target_compatible_with = [
        "@platforms//os:linux",
    ],
    deps = [
        "@anyhow",
        "@clap",
        "@env_logger",
        "@log",
        "@socket2",
        "@tokio",
        "@tokio-util",
        "@which",
    ],
)

sh_test(
    name = "ntest-e2e",
    srcs = ["ntest-e2e.sh"],
    args = [
        "$(location :ntest)",
        "MANUAL_BUILD_REQUIRED",
        "$(location @netsim//:netsimd)",
        "--dry-run",
    ],
    data = [
        ":ntest",
        "@netsim//:netsimd",
    ],
    tags = [
        "exclusive",
        "manual",
        "no-sandbox",
    ],
)
