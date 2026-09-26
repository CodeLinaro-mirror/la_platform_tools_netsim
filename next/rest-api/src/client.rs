// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Async Rust REST Client for Netsim daemon.

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{ListApResponse, ListDeviceResponse, ListLinkResponse};

/// Client for interacting with the Netsim REST API over HTTP/1.1.
#[derive(Clone, Debug)]
pub struct NetsimRestClient {
    addr: String,
}

impl NetsimRestClient {
    /// Creates a new `NetsimRestClient` targeting `host:port`.
    ///
    /// `host` must be a hostname or IP literal without a port (e.g.
    /// `"localhost"`, `"127.0.0.1"`, `"::1"`, or `"[::1]"`).
    pub fn new(host: &str, port: u16) -> Self {
        let addr = if host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("[{host}]:{port}")
        } else {
            format!("{host}:{port}")
        };
        Self { addr }
    }

    /// Sends a raw HTTP/1.1 GET request to the Netsim daemon and returns status
    /// code and response body.
    pub async fn send_request(&self, path: &str) -> Result<(u16, String), String> {
        let mut stream = tokio::net::TcpStream::connect(&self.addr)
            .await
            .map_err(|e| format!("Failed to connect to {}: {}", self.addr, e))?;

        let req =
            format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, self.addr);
        stream.write_all(req.as_bytes()).await.map_err(|e| e.to_string())?;

        let mut response_buf = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.read_to_end(&mut response_buf),
        )
        .await
        .map_err(|_e| "Request timed out".to_string())?
        .map_err(|e| e.to_string())?;

        let resp_str = String::from_utf8_lossy(&response_buf);
        let mut lines = resp_str.lines();
        let status_line = lines.next().ok_or_else(|| "Empty HTTP response".to_string())?;

        let status_code = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or_else(|| "Malformed HTTP status line".to_string())?;

        let (_, body) = resp_str
            .split_once("\r\n\r\n")
            .ok_or_else(|| "Invalid HTTP response: missing header/body separator".to_string())?;
        Ok((status_code, body.to_string()))
    }

    /// Generic helper to send GET request and deserialize JSON response.
    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let (status, body) = self.send_request(path).await?;
        if (200..300).contains(&status) {
            serde_json::from_str(&body).map_err(|e| e.to_string())
        } else {
            Err(format!("HTTP Error {}: {}", status, body))
        }
    }

    /// Convenience method to retrieve daemon version string.
    pub async fn get_version(&self) -> Result<String, String> {
        let val: serde_json::Value = self.fetch_json("/v1/version").await?;
        val.get("version")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| "Missing 'version' field in /v1/version response".to_string())
    }

    /// Convenience method to list devices.
    pub async fn get_devices(&self) -> Result<ListDeviceResponse, String> {
        self.fetch_json("/v1/devices").await
    }

    /// Convenience method to list links.
    pub async fn get_links(&self) -> Result<ListLinkResponse, String> {
        self.fetch_json("/v1/links").await
    }

    /// Convenience method to list APs.
    pub async fn get_aps(&self) -> Result<ListApResponse, String> {
        self.fetch_json("/v1/aps").await
    }
}
