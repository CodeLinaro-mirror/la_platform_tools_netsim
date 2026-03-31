// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    net::{Ipv6Addr, SocketAddr},
};

use bytes::BytesMut;
use device_actor::DeviceClient;
use http::{header::CONNECTION, Response, StatusCode};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::{
    io::AsyncReadExt,
    net::{TcpSocket, TcpStream},
};
use tracing::{error, info, warn};

use crate::{
    error::ServerError,
    handshake::{handle_websocket_handshake, write_http_response},
    transport::{run_websocket_transport, setup_virtual_chip},
};

const TARGET_PATH: &str = "/v1/websocket/bt";
const MAX_HTTP_BUFFER_SIZE: usize = 2048;
const CHANNEL_CAPACITY: usize = 100;

pub async fn run(websocket_port: u16, device_client: DeviceClient) {
    let addr = SocketAddr::from((Ipv6Addr::UNSPECIFIED, websocket_port));

    // Use socket2 to create an IPv6 socket and set dual-stack mode
    let socket = match Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP)) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create socket for WebSocket server: {e}");
            return;
        }
    };

    if let Err(e) = socket.set_only_v6(false) {
        warn!("Failed to set IPV6_V6ONLY=0 for WebSocket server: {e}");
    }

    if let Err(e) = socket.set_nonblocking(true) {
        error!("Failed to set O_NONBLOCK for WebSocket server: {e}");
        return;
    }

    // Convert to tokio::net::TcpSocket for binding and listening
    let tokio_socket = TcpSocket::from_std_stream(socket.into());

    if let Err(e) = tokio_socket.set_reuseaddr(true) {
        warn!("Failed to set SO_REUSEADDR for WebSocket server: {e}");
    }

    if let Err(e) = tokio_socket.bind(addr) {
        error!("Failed to bind WebSocket server to {addr}: {e}");
        return;
    }

    let listener = match tokio_socket.listen(128) {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to listen on {addr}: {e}");
            return;
        }
    };

    info!("WebSocket server is listening on: {websocket_port}");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("WebSocket client address: {addr}");
                let device_client = device_client.clone();
                tokio::spawn(async move {
                    handle_websocket_client(stream, addr, device_client).await;
                });
            }
            Err(e) => {
                warn!("Error accepting WebSocket connection: {e}");
            }
        }
    }
}

async fn handle_websocket_client(stream: TcpStream, addr: SocketAddr, device_client: DeviceClient) {
    // 1. Perform the WebSocket Handshake (RFC 6455, Section 1.3)
    let (websocket_reader, websocket_writer, queries) = match perform_handshake(stream).await {
        Ok(res) => res,
        Err(e) => {
            warn!("WebSocket handshake failed for {addr}: {e}");
            return;
        }
    };

    let (stream_tx, stream_rx) =
        tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(CHANNEL_CAPACITY);
    let (sink_tx, sink_rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(CHANNEL_CAPACITY);

    // 2. Register the virtual chip with the device actor
    let device_id =
        match setup_virtual_chip(&device_client, queries, addr, stream_rx, sink_tx).await {
            Ok(id) => id,
            Err(e) => {
                warn!("Failed to setup virtual chip for {addr}: {e}");
                return;
            }
        };

    // 3. Start the WebSocket transport loops (Read/Write/Forward)
    run_websocket_transport(
        websocket_reader,
        websocket_writer,
        addr,
        stream_tx,
        sink_rx,
        device_id,
        device_client,
    )
    .await;
}

/// Performs the initial HTTP handshake and upgrades to WebSocket.
///
/// Ref: RFC 6455, Section 1.3 - Opening Handshake
async fn perform_handshake(
    mut stream: TcpStream,
) -> Result<
    (
        tungstenite::WebSocket<std::net::TcpStream>,
        tungstenite::WebSocket<std::net::TcpStream>,
        HashMap<String, String>,
    ),
    ServerError,
> {
    let mut buffer = BytesMut::with_capacity(MAX_HTTP_BUFFER_SIZE);
    // Wait until we have a full HTTP request or hit MAX_HTTP_BUFFER_SIZE
    loop {
        let n = stream.read_buf(&mut buffer).await?;
        if n == 0 {
            if buffer.len() == buffer.capacity() {
                let response = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(CONNECTION, "close")
                    .body(Vec::new())?;
                let _ = write_http_response(&mut stream, response).await;
                return Err(ServerError::HandshakeFailed("Request too large".to_string()));
            } else {
                return Err(ServerError::HandshakeFailed(
                    "Client disconnected before sending full request".to_string(),
                ));
            }
        }

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse(&buffer) {
            Ok(httparse::Status::Complete(_)) => {
                break;
            }
            Err(err) => {
                let response = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(CONNECTION, "close")
                    .body(Vec::new())?;
                let _ = write_http_response(&mut stream, response).await;
                return Err(err.into());
            }
            Ok(httparse::Status::Partial) => {}
        }
    }

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    req.parse(&buffer).expect("validated parse is complete");

    // 1. Path validation and query extraction
    let path_str = req
        .path
        .ok_or_else(|| ServerError::HandshakeFailed("Missing HTTP request path".to_string()))?;

    let (base_path, query_string_opt) = match path_str.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (path_str, None),
    };

    if base_path != TARGET_PATH {
        let response = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(CONNECTION, "close")
            .body(Vec::new())?;
        let _ = write_http_response(&mut stream, response).await;
        return Err(ServerError::InvalidPath(base_path.to_string()));
    }

    let mut queries = HashMap::new();
    if let Some(query_string) = query_string_opt {
        queries = query_string
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
    }

    let response = handle_websocket_handshake(&req)?;

    // Write the upgrade response
    write_http_response(&mut stream, response).await?;

    // Transition to standard blocking stream for tungstenite
    let std_stream = stream.into_std()?;
    if let Err(e) = std_stream.set_nonblocking(false) {
        error!("Failed to set std stream to blocking: {e}");
        return Err(ServerError::Io(e));
    }

    // Initialize WebSocket readers and writers
    let websocket_writer = tungstenite::WebSocket::from_raw_socket(
        std_stream.try_clone()?,
        tungstenite::protocol::Role::Server,
        None,
    );
    let websocket_reader = tungstenite::WebSocket::from_raw_socket(
        std_stream,
        tungstenite::protocol::Role::Server,
        None,
    );

    Ok((websocket_reader, websocket_writer, queries))
}
