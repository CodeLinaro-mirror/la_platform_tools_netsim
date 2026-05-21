// Copyright 2025-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

fn main() {
    cxx_build::bridge("src/ffi.rs")
        .file("src/crypto_ffi.cc")
        .include("src")
        .flag_if_supported("-std=c++17")
        .compile("hostap_crypto");

    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=src/crypto_ffi.cc");
    println!("cargo:rerun-if-changed=src/crypto_ffi.h");

    // Link against libcrypto (BoringSSL/OpenSSL)
    // In Android build this is handled by Soong/Bazel, but for Cargo:
    println!("cargo:rustc-link-lib=crypto");
}
