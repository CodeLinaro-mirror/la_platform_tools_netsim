load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

DEPS = [
    "//next/actor-framework",
    "//next/capture-api",
    "//next/device-api",
    "//next/model",
    "@futures",
    "@tokio",
    "@serde",
    "@log",
    "@thiserror",
]

netsim_rust_library(
    name = "device-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_features = [],
    crate_name = "device_actor",
    crate_root = "src/lib.rs",
    edition = "2021",
    enable_clippy = False,
    enable_doc_test = False,
    proc_macro_deps = [
        "@async-trait",
    ],
    strict_warnings = False,
    deps = DEPS,
)

[
    rust_test(
        name = test_file.replace("tests/", "").replace(".rs", ""),
        srcs = [
            test_file,
            "tests/common/mod.rs",
        ],
        edition = "2021",
        # enable_clippy = False,
        # enable_doc_test = False,
        proc_macro_deps = [
            "@async-trait",
        ],
        deps = [
            ":device-actor",
            "//next/client",
        ] + DEPS,
    )
    for test_file in [
        "tests/add_chip_integration_test.rs",
        "tests/device_lifecycle_test.rs",
        "tests/integration_test.rs",
        "tests/server_shutdown_test.rs",
    ]
]
