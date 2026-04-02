// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! A library for reading and writing pcap (packet capture) files in Rust.
//!
//! This crate provides an asynchronous API for working with pcap files,
//! allowing you to read and write packet capture data efficiently.
//! It supports both reading from and writing to pcap files, and it
//! handles the parsing and serialization of pcap headers and packet records.
//!
//! # Features
//!
//! * **Asynchronous API:** Built on top of Tokio, enabling efficient asynchronous
//!   reading and writing of pcap files.
//! * **Zero-copy:** Uses the `zerocopy` crate for zero-cost conversions between
//!   structs and byte slices, improving performance.
//! * **Standard pcap format:**  Supports the standard pcap file format, ensuring
//!   compatibility with other pcap tools.
//!

/// This module contains the core functionality for reading and writing pcap files.
pub mod pcap;
