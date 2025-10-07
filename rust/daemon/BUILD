load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//:__subpackages__"])

exports_files(["src/ffi.rs"])

rust_library(
    name = "netsim_daemon",
    srcs = glob(["src/**/*.rs"]),
    aliases = {
        "//rust/hostapd-rs:netsim_hostapd_rs": "hostapd_rs",
        "//rust/http-proxy:netsim_http_proxy": "http_proxy",
        "//rust/libslirp-rs:netsim_libslirp_rs": "libslirp_rs",
    },
    compile_data = [
        "src/uwb/sample_ranging_data.csv",
    ],
    crate_name = "netsim_daemon",
    crate_root = "src/lib.rs",
    rustc_flags = [
        "--cfg=feature=\"local_ssl\"",
    ],
    deps = [
        "//rust/common:netsim_common",
        "//rust/hostapd-rs:netsim_hostapd_rs",
        "//rust/http-proxy:netsim_http_proxy",
        "//rust/libslirp-rs:netsim_libslirp_rs",
        "//rust/packets:netsim_packets",
        "//rust/proto-cf",
        "@anyhow",
        "@bytes",
        "@clap",
        "@cxx.rs//:cxx",
        "@data-encoding",
        "@futures",
        "@futures-channel",
        "@futures-executor",
        "@futures-util",
        "@glam",
        "@grpcio",
        "@http",
        "@httparse",
        "@libc",
        "@log",
        "@pdl-runtime",
        "@pica",
        "@protobuf-json-mapping",
        "@protobuf-rust",
        "@rand",
        "@regex",
        "@socket2",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@tungstenite",
    ],
)
