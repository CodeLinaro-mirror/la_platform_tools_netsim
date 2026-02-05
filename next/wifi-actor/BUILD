load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

netsim_rust_library(
    name = "wifi-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "wifi_actor",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        "//next/actor-framework",
        "//next/ap-actor",
        "//next/client",
        "//next/model",
        "//next/packets",
        "//next/slirp-actor",
        "//proto",
        "@bytes",
        "@futures",
        "@log",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@zerocopy",
    ],
)

rust_test(
    name = "integration_test",
    size = "small",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/integration_test.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        ":testing",
        "//next/actor-framework:testing",
        "//next/ap-actor:testing",
        "//next/client:testing",
        "//next/device-actor:testing",
        "//next/device-api:testing",
        "//next/model:testing",
        "//next/packets:testing",
        "//next/slirp-actor:testing",
        "@bytes",
        "@futures",
        "@libc",
        "@log",
        "@tokio",
        "@tokio-stream",
        "@zerocopy",
    ],
)

rust_test(
    name = "packet_routing_test",
    srcs = [
        "tests/hwsim_helper.rs",
        "tests/packet_routing_test.rs",
        "tests/world.rs",
    ],
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        ":testing",
        "//next/actor-framework:testing",
        "//next/ap-actor:testing",
        "//next/client:testing",
        "//next/device-actor:testing",
        "//next/device-api:testing",
        "//next/model:testing",
        "//next/packets:testing",
        "//next/slirp-actor:testing",
        "@bytes",
        "@futures",
        "@tokio",
        "@tokio-stream",
        "@zerocopy",
    ],
)

rust_test(
    name = "lifecycle_test",
    srcs = [
        "tests/hwsim_helper.rs",
        "tests/lifecycle_test.rs",
        "tests/world.rs",
    ],
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        ":testing",
        "//next/actor-framework:testing",
        "//next/ap-actor:testing",
        "//next/client:testing",
        "//next/device-actor:testing",
        "//next/device-api:testing",
        "//next/model:testing",
        "//next/packets:testing",
        "//next/slirp-actor:testing",
        "@bytes",
        "@futures",
        "@tokio",
        "@tokio-stream",
        "@zerocopy",
    ],
)

rust_test(
    name = "ftm_test",
    srcs = ["tests/ftm_test.rs"],
    edition = "2021",
    deps = [
        ":testing",
        "//next/model:testing",
        "//next/packets:testing",
        "@zerocopy",
    ],
)
