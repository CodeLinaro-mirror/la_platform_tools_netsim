// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! This module defines the Proxy error types.

use std::{io, net::SocketAddr};

use thiserror::Error;

/// Represents all possible errors that can occur in the http-proxy crate.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    /// The HTTP request-line is malformed.
    #[error("Malformed request line: '{0}'")]
    MalformedRequestLine(String),
    /// The 'Host' header is missing.
    #[error("Mandatory 'Host' header is missing")]
    MissingHostHeader,
    /// The proxy server returned an error.
    #[error("Connection to {0} failed: {1}")]
    ConnectionError(SocketAddr, String),
    /// The proxy configuration string is malformed.
    #[error("Malformed configuration string")]
    MalformedConfigString,
    /// The port number in the proxy configuration is invalid.
    #[error("Invalid port number: {0}")]
    InvalidPortNumber(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The host in the proxy configuration is invalid.
    #[error("Invalid host: {0}")]
    InvalidHost(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A type alias for `Result` where the error type is this crate's `Error`.
pub type Result<T> = std::result::Result<T, Error>;
