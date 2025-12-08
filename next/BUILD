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
        "//next/" + pkg
        for pkg in subpackages(include = ["**"])
    ],
)
