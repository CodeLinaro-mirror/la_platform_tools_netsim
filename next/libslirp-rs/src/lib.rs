// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! This crate is a wrapper for libslirp C library.
//!
//! All calls into libslirp are routed to and handled by a dedicated
//! thread.
//!
//! Rust struct LibslirpConfig for conversion between Rust and C types
//! (IpV4Addr, SocketAddrV4, etc.).
//!
//! Callbacks for libslirp send_packet are delivered on Channel.
mod libslirp;
mod libslirp_config;
// Keep libslirp_sys public to bypass dead code warnings for unused
// auto-generated FFI bindings.
pub mod libslirp_sys;

pub use libslirp::{LibSlirp, ProxyConnect, ProxyManager};
pub use libslirp_config::{lookup_host_dns, SlirpConfig};
