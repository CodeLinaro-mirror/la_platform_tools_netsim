// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # HTTP Proxy
//!
//! This crate provides a TCP proxy client that can be used to
//! establish connections to a target address through an HTTP proxy
//! server.
//!
//! The main component of this crate is the `Connector` struct, which
//! handles the CONNECT request handshake with the proxy, including
//! optional Basic authentication.
//!
//! The crate also includes a `Manager` struct that implements the
//! `ProxyManager` trait from `libslirp_rs`, allowing it to be used
//! with the `libslirp` library for managing TCP connections through
//! the proxy.
//!
//! ## Example
//!
//! ```
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() {
//!     let proxy_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
//!
//!     let connector = http_proxy::Connector::new(proxy_addr, None, None);
//! }
//! ```
//!
//! ## Features
//!
//! * **libslirp:** Enables integration with the `libslirp` library.
//!
//! ## Limitations
//!
//! * Currently only supports HTTP proxies.
//! * Usernames and passwords cannot contain `@` or `:`.
//TODO: Remove once implementation is complete

mod connector;
mod dns;
mod dns_manager;
mod error;
mod manager;

mod rewriter;
mod util;

pub use connector::*;
pub use dns_manager::*;
pub use error::{Error, Result};
pub use manager::*;
