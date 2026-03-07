load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/device-actor",
    "//next/device-api",
    "//next/model",
    "//proto",
    "@anyhow",
    "@bytes",
    "@futures",
    "@log",
    "@pdl-runtime",
    "@pica",
    "@tokio",
    "@tokio-util",
    "@tokio-stream",
    "@serde",
    "@glam",
]

netsim_rust_library(
    name = "uwb-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = ["testing"],
    crate_name = "uwb_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    test_deps = [
        "//next/testing",
        "@env_logger",
    ],
    deps = DEPS,
)
