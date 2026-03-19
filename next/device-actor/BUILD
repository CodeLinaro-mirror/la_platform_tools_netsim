load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/capture-api",
    "//next/device-api",
    "//next/model",
    "//next/link-api",
    "//next/common",
    "//proto",
    "@protobuf-json-mapping",
    "@futures",
    "@tokio",
    "@serde",
    "@tracing",
    "@thiserror",
    "@bytes",
]

netsim_rust_library(
    name = "device-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "device_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    test_deps = [
        "@mockall",
        "@serde_json",
        "@tokio-stream",
    ],
    deps = DEPS,
)
