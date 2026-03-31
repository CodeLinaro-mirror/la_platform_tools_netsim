// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # hostapd-rs
//!
//! This crate provides a Rust interface to the `hostapd` C library, allowing you to manage WiFi access points
//! and perform various wireless networking tasks directly from your Rust code.
//!
//! It consists of two main modules:
//!
//! * **`hostapd`:** This module provides a high-level and safe interface to interact with the `hostapd` process.
//!   It uses separate threads for managing the `hostapd` process and handling its responses, ensuring efficient
//!   and non-blocking communication.
//! * **`hostapd_sys`:** This module contains the low-level C FFI bindings to the `hostapd` library. It is
//!   automatically generated using `rust-bindgen` and provides platform-specific bindings for Linux, macOS, and Windows.
//!

pub mod hostapd;
pub mod hostapd_sys;
