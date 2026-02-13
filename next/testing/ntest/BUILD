load("@rules_rust//rust:defs.bzl", "rust_binary")

package(default_visibility = ["//visibility:public"])

rust_binary(
    name = "ntest",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
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
        "@which",
    ],
)
