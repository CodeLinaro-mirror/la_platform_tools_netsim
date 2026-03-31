// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Build script for linking `libslirp-rs` with dependencies.

/// The main function of the build script.
///
/// Configures the build process to link against `libslirp` and other
/// OS-dependent libraries.
pub fn main() {
    let objs_path = std::env::var("OBJS_PATH").unwrap_or("../objs".to_string());

    println!("cargo:rustc-link-search={objs_path}/archives");
    println!("cargo:rustc-link-search={objs_path}/lib64");
    println!("cargo:rustc-link-lib=libslirp");
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=glib2_linux-x86_64");
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    println!("cargo:rustc-link-lib=glib2_darwin-x86_64");
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    println!("cargo:rustc-link-lib=glib2_darwin-aarch64");
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=glib2_windows_msvc-x86_64");
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=iphlpapi");
}
