load(
    "@rules_rust//rust:defs.bzl",
    "rustfmt_test",
)

exports_files([
    "rustfmt.toml",
])

rustfmt_test(
    name = "fmt",
    targets = [
        "//next/bluetooth",
        "//next/devices",
        "//next/netsim-api",
        "//next/packets",
        "//next/proto",
        "//next/rootcanal",
    ],
)
