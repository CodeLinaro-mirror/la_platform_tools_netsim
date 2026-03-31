// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Build script for linking `hostapd-rs` with dependencies.

/// Build script for linking `hostapd-rs` with the `hostapd` C library and OS specific dependencies.
pub fn main() {
    let objs_path = std::env::var("OBJS_PATH").unwrap_or("../objs".to_string());

    // Shared dependencies
    println!("cargo:rustc-link-search={objs_path}/archives");
    println!("cargo:rustc-link-lib=hostapd");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=android-emu-base");
    println!("cargo:rustc-link-lib=android-emu-utils");
    println!("cargo:rustc-link-lib=logging-base");
    println!("cargo:rustc-link-lib=android-emu-base-logging");
    // Linux and Mac dependencies
    #[cfg(unix)]
    {
        println!("cargo:rustc-link-search={objs_path}/lib64");
        println!("cargo:rustc-link-lib=c++");
    }
    // Windows dependencies
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-lib=crypto_asm_lib");
        println!("cargo:rustc-link-search={objs_path}/msvc-posix-compat/msvc-compat-layer");
        println!("cargo:rustc-link-lib=msvc-posix-compat");
        println!("cargo:rustc-link-search=C:/Windows/System32");
        println!("cargo:rustc-link-lib=vcruntime140");
    }
}
