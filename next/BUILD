load(
    "@rules_rust//rust:defs.bzl",
    "rustfmt_test",
)

exports_files([
    "rustfmt.toml",
])

EXCLUDED_PACKAGES = [
    "actor-framework",
    "capture-actor",
    "cell",
    "client",
    "device-actor",
    "grpc-server",
    "modem-rs",
    "types",
]

rustfmt_test(
    name = "fmt",
    tags = ["manual"],
    targets = [
        "//next/" + pkg
        for pkg in subpackages(include = ["**"])
        if pkg not in EXCLUDED_PACKAGES
    ],
)
