// Copyright 2025 Google LLC
//=============================================================================
// src/transport/adapters.rs - Adapters for existing PacketStream implementations
//=============================================================================

use crate::error::{PacketStreamError, Result};
use crate::models::ChipInfo;
use crate::streams::InitInfo;
use crate::transport::traits::{PacketSink, PacketStream, TransportListener};
use crate::types::StreamAddress;
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::SinkExt;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::{TcpListener, UnixListener};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// TCP listener adapter
pub struct TcpTransportListener {
    inner: TcpListener,
    local_addr: SocketAddr,
}

impl TcpTransportListener {
    pub async fn bind(addr: &str, port: u16) -> Result<Self> {
        let bind_addr = format!("{addr}:{port}");
        let listener = TcpListener::bind(&bind_addr).await?;
        let local_addr = listener.local_addr()?;

        Ok(Self { inner: listener, local_addr })
    }
}

#[async_trait]
impl TransportListener for TcpTransportListener {
    async fn accept(&mut self) -> Result<(PacketStream, PacketSink, ChipInfo)> {
        let (stream, _) = self.inner.accept().await?;
        let framed = Framed::new(stream, LengthDelimitedCodec::new());
        let (sink, mut stream) = framed.split();

        // Perform the init_info handshake
        let first_message = stream.next().await.ok_or(PacketStreamError::ConnectionClosed)??;
        let init_info: InitInfo = serde_json::from_slice(&first_message).map_err(|e| {
            PacketStreamError::Protocol(crate::error::ProtocolError::InvalidFormat(format!(
                "Failed to parse init_info: {e}"
            )))
        })?;

        let stream = stream.map(|item| item.map(|b| b.freeze()).map_err(PacketStreamError::Io));
        let sink = sink.sink_map_err(PacketStreamError::Io);

        Ok((Box::pin(stream), Box::pin(sink), init_info.chip_info))
    }

    fn local_addr(&self) -> Result<StreamAddress> {
        Ok(StreamAddress::Tcp(self.local_addr))
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Unix Domain Socket listener adapter
pub struct UdsTransportListener {
    inner: UnixListener,
    path: PathBuf,
}

impl UdsTransportListener {
    pub async fn bind(path: &str) -> Result<Self> {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() {
            std::fs::remove_file(&path_buf)?;
        }
        let listener = UnixListener::bind(&path_buf)?;
        Ok(Self { inner: listener, path: path_buf })
    }
}

#[async_trait]
impl TransportListener for UdsTransportListener {
    async fn accept(&mut self) -> Result<(PacketStream, PacketSink, ChipInfo)> {
        let (stream, _) = self.inner.accept().await?;
        let framed = Framed::new(stream, LengthDelimitedCodec::new());
        let (sink, mut stream) = framed.split();

        // Perform the init_info handshake
        let first_message = stream.next().await.ok_or(PacketStreamError::ConnectionClosed)??;
        let init_info: InitInfo = serde_json::from_slice(&first_message).map_err(|e| {
            PacketStreamError::Protocol(crate::error::ProtocolError::InvalidFormat(format!(
                "Failed to parse init_info: {e}"
            )))
        })?;

        let stream = stream.map(|item| item.map(|b| b.freeze()).map_err(PacketStreamError::Io));
        let sink = sink.sink_map_err(PacketStreamError::Io);

        Ok((Box::pin(stream), Box::pin(sink), init_info.chip_info))
    }

    fn local_addr(&self) -> Result<StreamAddress> {
        Ok(StreamAddress::Uds(self.path.clone()))
    }

    async fn shutdown(&mut self) -> Result<()> {
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
        Ok(())
    }
}
