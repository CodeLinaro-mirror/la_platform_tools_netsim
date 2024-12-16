// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::Error;
use base64::{engine::general_purpose, Engine as _};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

const HTTP_VERSION: &str = "1.1";

pub type Result<T> = core::result::Result<T, Error>;

/// Establishes a TCP connection to a target address through an HTTP proxy.
///
/// The `Connector` handles the CONNECT request handshake with the proxy, including
/// optional Basic authentication.
#[derive(Clone)]
pub struct Connector {
    proxy_addr: SocketAddr,
    username: Option<String>,
    password: Option<String>,
}

impl Connector {
    pub fn new(proxy_addr: SocketAddr, username: Option<String>, password: Option<String>) -> Self {
        Connector { proxy_addr, username, password }
    }

    pub async fn connect(&self, addr: SocketAddr) -> Result<TcpStream> {
        let mut stream = TcpStream::connect(self.proxy_addr).await?;

        // Construct the CONNECT request
        let mut request = format!("CONNECT {} HTTP/{}\r\n", addr.to_string(), HTTP_VERSION);

        // Authentication
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            let encoded_auth = base64_encode(format!("{}:{}", username, password).as_bytes());
            let auth_header = format!(
                "Proxy-Authorization: Basic {}\r\n",
                String::from_utf8_lossy(&encoded_auth)
            );
            // Add the header to the request
            request.push_str(&auth_header);
        }

        // Add the final CRLF
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).await?;

        // Read the proxy's response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await?;
        if response.starts_with(&format!("HTTP/{} 200", HTTP_VERSION)) {
            Ok(reader.into_inner())
        } else {
            Err(Error::ConnectionError(addr, response.trim_end_matches("\r\n").to_string()))
        }
    }
}

fn base64_encode(src: &[u8]) -> Vec<u8> {
    general_purpose::STANDARD.encode(src).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::net::{lookup_host, TcpListener};

    #[tokio::test]
    async fn test_connect() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let addr: SocketAddr = lookup_host("localhost:8000").await.unwrap().next().unwrap();

        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            // Server expects a client greeting with no auth methods
            let expected_greeting = format!("CONNECT {} HTTP/1.1\r\n", &addr);

            let mut reader = BufReader::new(stream);
            let mut line = String::new();

            reader.read_line(&mut line).await.unwrap();

            assert_eq!(line, expected_greeting);

            // Server sends a response with no auth method selected
            let response = "HTTP/1.1 200 Connection established\r\n\r\n";
            let mut stream = reader.into_inner();
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let client = Connector::new(proxy_addr, None, None);

        client.connect(addr).await.unwrap();

        handle.await.unwrap(); // Wait for the task to complete

        Ok(())
    }

    #[tokio::test]
    async fn test_connect_with_auth() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let addr: SocketAddr = lookup_host("localhost:8000").await.unwrap().next().unwrap();

        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Server expects a client greeting with auth header
            let expected_greeting = format!(
                "CONNECT {} HTTP/1.1\r\nProxy-Authorization: Basic dXNlcjpwYXNzd29yZA==\r\n\r\n",
                &addr
            );

            let mut buf = [0; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            let actual_greeting = String::from_utf8_lossy(&buf[..n]);

            assert_eq!(actual_greeting, expected_greeting);

            // Server sends a response
            let response = "HTTP/1.1 200 Connection established\r\n\r\n";

            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let client = Connector::new(proxy_addr, Some("user".into()), Some("password".into()));

        client.connect(addr).await.unwrap();

        handle.await.unwrap(); // Wait for the task to complete

        Ok(())
    }

    #[test]
    fn test_proxy_base64_encode_success() {
        let input = b"hello world";
        let encoded = base64_encode(input);
        assert_eq!(encoded, b"aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn test_proxy_base64_encode_empty_input() {
        let input = b"";
        let encoded = base64_encode(input);
        assert_eq!(encoded, b"");
    }
}
