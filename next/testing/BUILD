load("//next:defs.bzl", "netsim_rust_library")

netsim_rust_library(
    name = "netsim-testing",
    testonly = True,
    srcs = glob(["src/**/*.rs"]),
    crate_features = ["testing"],
    crate_name = "netsim_testing",
    edition = "2021",
    enable_rustfmt = False,
    visibility = ["//next:__subpackages__"],
    deps = [
        "//next/common",
        "@bytes",
        "@futures",
        "@tokio",
        "@tokio-stream",
    ],
)
