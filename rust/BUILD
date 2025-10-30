load(
    "@rules_rust//rust:defs.bzl",
    "rustfmt_test",
)

exports_files([
    "rustfmt.toml",
    "netsimd.cc",
])

rustfmt_test(
    name = "fmt",
    targets = [
        "//rust/bluetooth",
        "//rust/capture:netsim_capture",
        "//rust/cli:netsim_cli",
        "//rust/common:netsim_common",
        "//rust/daemon:netsim_daemon",
        "//rust/devices",
        "//rust/hostapd-rs:netsim_hostapd_rs",
        "//rust/http-proxy:netsim_http_proxy",
        "//rust/libslirp-rs:netsim_libslirp_rs",
        "//rust/netsim-api",
        "//rust/packets:netsim_packets",
        "//rust/proto-cf",
        "//rust/rootcanal",
    ],
)

genrule(
    name = "cxx-bridge-header",
    outs = ["cxx.h"],
    cmd = "$(location @cxx.rs//:codegen) --header >> $(OUTS)",
    tools = ["@cxx.rs//:codegen"],
    visibility = ["//:__pkg__"],
)
