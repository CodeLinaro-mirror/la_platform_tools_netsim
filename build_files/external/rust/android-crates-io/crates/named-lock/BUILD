load(
    "@rules_rust//rust:defs.bzl",
    "rust_library",
)

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "named-lock",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "named_lock",
    edition = "2018",
    deps = [
        "@once_cell",
        "@parking_lot",
        "@thiserror",
    ] + select({
        "@platforms//os:linux": ["@libc"],
        "@platforms//os:macos": ["@libc"],
        "@platforms//os:windows": ["@windows-sys"],
        "//conditions:default": [],
    }),
)
