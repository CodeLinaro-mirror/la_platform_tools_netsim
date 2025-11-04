load(
    "@rules_rust//rust:defs.bzl",
    "rustfmt_test",
)

exports_files([
    "rustfmt.toml",
])

rustfmt_test(
    name = "fmt",
    tags = ["manual"],
    targets = [
        "//next/bluetooth",
        "//next/common",
        "//next/devices",
        "//next/hostapd-rs",
        "//next/http-proxy",
        "//next/libslirp-rs",
        "//next/modem-rs-derive",
        # "//next/modem-rs",
        "//next/netsim-api",
        "//next/netsim-next",
        "//next/packet-stream",
        "//next/packets",
        "//next/proto",
        "//next/rootcanal",
        "//next/wifi",
    ],
)
