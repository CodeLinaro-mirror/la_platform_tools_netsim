// Copyright 2024-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

/// Trait for handling proxy connection results.
pub trait ProxyConnect: Send {
    /// Notifies slirp about the result of a proxy connection attempt.
    fn proxy_connect(&self, fd: i32, addr: SocketAddr);
}

/// HTTP Proxy callback trait
pub trait ProxyManager: Send + Sync {
    /// Attempts to establish a connection through the proxy.
    fn try_connect(
        &self,
        sockaddr: SocketAddr,
        connect_id: usize,
        connect_func: Box<dyn ProxyConnect + Send>,
    ) -> bool;
    /// Removes a proxy connection.
    fn remove(&self, connect_id: usize);
}
