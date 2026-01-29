load("@cxx.rs//tools/bazel:rust_cxx_bridge.bzl", "rust_cxx_bridge")
load("@rules_cc//cc:defs.bzl", "cc_library")
load("@rules_rust//rust:defs.bzl", "rust_test")
load("//next:defs.bzl", "netsim_rust_library")

package(default_visibility = ["//visibility:public"])

cc_library(
    name = "hostap_crypto_shim",
    srcs = ["src/crypto_ffi.cc"],
    hdrs = ["src/crypto_ffi.h"],
    defines = [
        "CONFIG_OPENSSL_INTERNAL_AES_WRAP",
    ],
    includes = ["src"],
    deps = [
        "@boringssl//:crypto",
        "@cxx.rs//:core",
    ],
)

netsim_rust_library(
    name = "ap-actor",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "ap_actor",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
    ],
    deps = [
        ":ap_actor_cxx",
        ":hostap_crypto_shim",
        "//next/actor-framework",
        "//next/model",
        "//next/packets",
        "@aes",
        "@bytes",
        "@ccm",
        "@cxx.rs//:cxx",
        "@futures",
        "@log",
        "@mockall",
        "@serde",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
        "@zerocopy",
    ],
)

# Architecture Note:
# ap-actor uses C++ BoringSSL via cxx bridge for WPA/EAPOL Authentication to ensure
# strict compliance and correctness with hostapd/wpa_supplicant behavior.
# Native Rust crypto crates (hmac, sha1, etc) required for Auth are not available
# in the Netsim workspace configuration.
#
# wifi-actor (Data Plane) uses the available 'ccm' and 'aes' Rust crates for
# high-performance, zero-copy packet encryption/decryption.
rust_cxx_bridge(
    name = "ap_actor_bridge",
    src = "src/ffi.rs",
    deps = [":hostap_crypto_shim"],
)

cc_library(
    name = "ap_actor_cxx",
    srcs = [":ap_actor_bridge"],
    deps = [
        ":hostap_crypto_shim",
        "@cxx.rs//:core",
    ],
)

rust_test(
    name = "integration-test",
    size = "small",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    edition = "2021",
    deps = [
        ":testing",
        "//next/actor-framework:testing",
        "//next/model",
        "//next/packets:testing",
        "@bytes",
        "@env_logger",
        "@futures",
        "@log",
        "@rand",
        "@tokio",
        "@tokio-stream",
        "@zerocopy",
    ],
)
