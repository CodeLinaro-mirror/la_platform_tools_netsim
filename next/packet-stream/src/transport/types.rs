// Copyright 2025 Google LLC
//=============================================================================
// src/transport/types.rs - Transport type enumeration and factories
//=============================================================================

use std::collections::HashMap;

use futures::{stream::StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::{
    error::{PacketStreamError, Result},
    transport::{
        adapters::{TcpTransportListener, UdsTransportListener},
        traits::{PacketSink, PacketStream},
    },
};

/// Transport configuration that supports multiple connection types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransportType {
    Tcp { addr: String, port: u16 },
    Uds { path: String },
    Fd { in_fd: i32, out_fd: Option<i32> },
}

impl TransportType {
    pub fn tcp(addr: impl Into<String>, port: u16) -> Self {
        TransportType::Tcp { addr: addr.into(), port }
    }

    pub fn uds(path: impl Into<String>) -> Self {
        TransportType::Uds { path: path.into() }
    }

    pub fn fd(in_fd: i32, out_fd: Option<i32>) -> Self {
        TransportType::Fd { in_fd, out_fd }
    }

    pub async fn create_listener(&self) -> Result<super::Listener> {
        match self {
            TransportType::Tcp { addr, port } => {
                let listener = TcpTransportListener::bind(addr, *port).await?;
                Ok(super::Listener::Tcp(listener))
            }
            TransportType::Uds { path } => {
                let listener = UdsTransportListener::bind(path).await?;
                Ok(super::Listener::Uds(listener))
            }
            TransportType::Fd { .. } => Err(PacketStreamError::InvalidConfig(
                "File descriptors cannot create listeners, use create_stream() instead".to_string(),
            )),
        }
    }

    pub async fn create_stream(&self) -> Result<(PacketStream, PacketSink)> {
        match self {
            TransportType::Tcp { addr, port } => {
                let stream = TcpStream::connect(format!("{addr}:{port}")).await?;
                let framed = Framed::new(stream, LengthDelimitedCodec::new());
                let (sink, stream) = framed.split();
                let stream =
                    stream.map(|item| item.map(|b| b.freeze()).map_err(PacketStreamError::Io));
                let sink = sink.sink_map_err(PacketStreamError::Io);
                Ok((Box::pin(stream), Box::pin(sink)))
            }
            TransportType::Uds { path } => {
                let stream = tokio::net::UnixStream::connect(path).await?;
                let framed = Framed::new(stream, LengthDelimitedCodec::new());
                let (sink, stream) = framed.split();
                let stream =
                    stream.map(|item| item.map(|b| b.freeze()).map_err(PacketStreamError::Io));
                let sink = sink.sink_map_err(PacketStreamError::Io);
                Ok((Box::pin(stream), Box::pin(sink)))
            }
            TransportType::Fd { .. } => Err(PacketStreamError::InvalidConfig(
                "FD transport cannot be created directly, it must be inherited".to_string(),
            )),
        }
    }

    pub fn description(&self) -> String {
        match self {
            TransportType::Tcp { addr, port } => format!("TCP {addr}:{port}"),
            TransportType::Uds { path } => format!("UDS {path}"),
            TransportType::Fd { in_fd, out_fd } => match out_fd {
                Some(out_fd) => format!("FD {in_fd}:{out_fd}"),
                None => format!("FD {in_fd}"),
            },
        }
    }

    pub fn supports_listener(&self) -> bool {
        matches!(self, TransportType::Tcp { .. } | TransportType::Uds { .. })
    }

    pub fn supports_stream(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerConfig {
    pub listeners: HashMap<String, TransportType>,
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ListenerConfig {
    pub fn new() -> Self {
        Self { listeners: HashMap::new() }
    }

    pub fn add_listener(&mut self, name: impl Into<String>, transport_type: TransportType) {
        self.listeners.insert(name.into(), transport_type);
    }

    pub fn default_config() -> Self {
        let mut config = Self::new();
        config.add_listener("tcp", TransportType::tcp("localhost", 8080));
        config.add_listener("uds", TransportType::uds("/tmp/packetstream.sock"));
        config
    }
}
