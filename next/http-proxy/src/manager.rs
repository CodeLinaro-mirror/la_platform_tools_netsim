// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::SocketAddr,
    sync::{Arc, mpsc},
    thread,
};

use bytes::Bytes;
use libslirp_rs::{ProxyConnect, ProxyManager};
use tokio::runtime::Runtime;
use tracing::{debug, warn};

use crate::{
    Connector, DnsManager, Result,
    util::{ProxyConfig, into_raw_descriptor},
};

/// # Manager
///
/// The `Manager` struct implements the `ProxyManager` trait from
/// `libslirp_rs`.  It is responsible for managing TCP connections
/// through an HTTP proxy using the `Connector` struct.
///
/// The `Manager` uses a `tokio::runtime::Runtime` to spawn tasks for
/// establishing proxy connections.  It takes a proxy configuration
/// string as input, which is parsed into a `ProxyConfig` to create a
/// `Connector` instance.
///
/// The `try_connect` method attempts to establish a connection to the
/// given `SocketAddr` through the proxy.  If successful, it calls the
/// `proxy_connect` function with the raw file descriptor of the
/// connected socket.
///
/// # Example
///
/// ```
/// use std::net::SocketAddr;
///
/// use libslirp_rs::ProxyConnect;
///
/// struct MyProxyConnect;
///
/// impl ProxyConnect for MyProxyConnect {
///     fn proxy_connect(&self, fd: i32, sockaddr: SocketAddr) {
///         // Handle the connected socket
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {}
/// ```
pub struct Manager {
    runtime: Arc<Runtime>,
    connector: Connector,
}

impl Manager {
    /// Creates a new `Manager` instance to handle HTTP proxy connections.
    ///
    /// This function initializes the proxy configuration, creates a
    /// `DnsManager` for IP-to-FQDN reverse lookup caching, and spawns a
    /// background thread to capture and process ethernet traffic for DNS
    /// caching.
    pub fn new(proxy: &str, rx_proxy_bytes: mpsc::Receiver<Bytes>) -> Result<Self> {
        let config = ProxyConfig::from_string(proxy)?;
        let dns_manager = Arc::new(DnsManager::new());

        let _ = thread::Builder::new().name("Dns Manager".to_string()).spawn(move || {
            while let Ok(bytes) = rx_proxy_bytes.recv() {
                dns_manager.add_from_ethernet_slice(&bytes);
            }
        });

        // We initialize a private, isolated tokio runtime for HTTP proxy tasks.
        // Sharing the main daemon runtime with high-throughput actors (like Slirp or
        // Bluetooth) can cause scheduling delays for HTTP reachability checks,
        // leading to PARTIAL_CONNECTIVITY status. Isolation ensures consistent
        // performance.
        let runtime = Arc::new(Runtime::new()?);

        Ok(Self {
            runtime,
            connector: Connector::new(config.addr, config.username, config.password),
        })
    }
}

impl ProxyManager for Manager {
    /// Attempts to establish a TCP connection to the given `sockaddr` through
    /// the proxy.
    ///
    /// This function spawns a new task in the `tokio` runtime to handle the
    /// connection process. If the connection is successful, it calls the
    /// `proxy_connect` function of the provided `ProxyConnect` object with
    /// the raw file descriptor of the connected socket.
    ///
    /// # Arguments
    ///
    /// * `sockaddr` - The target socket address to connect to.
    /// * `connect_id` - An identifier for the connection.
    /// * `connect_func` - A `ProxyConnect` object that will be called with the
    ///   connected socket.
    ///
    /// # Returns
    ///
    /// `true` if the connection attempt was initiated, `false` otherwise.
    fn try_connect(
        &self,
        sockaddr: SocketAddr,
        connect_id: usize,
        connect_func: Box<dyn ProxyConnect + Send>,
    ) -> bool {
        debug!("Connecting to {sockaddr:?} with connect ID {connect_id}");
        let connector = self.connector.clone();

        self.runtime.handle().spawn(async move {
            let fd = match connector.connect(sockaddr).await {
                Ok(tcp_stream) => into_raw_descriptor(tcp_stream),
                Err(e) => {
                    warn!("Failed to connect to proxy {}. {}", sockaddr, e);
                    -1
                }
            };
            connect_func.proxy_connect(fd, sockaddr);
        });

        true
    }

    /// Removes a connection with the given `connect_id`.
    ///
    /// Currently, this function only logs a debug message.
    ///
    /// # Arguments
    ///
    /// * `connect_id` - The identifier of the connection to remove.
    fn remove(&self, connect_id: usize) {
        debug!("Remove connect ID {}", connect_id);
    }
}
