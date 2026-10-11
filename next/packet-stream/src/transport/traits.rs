// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0
//=============================================================================
// src/transport/traits.rs - Transport traits
//=============================================================================
use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Sink, Stream};

use crate::{
    error::{PacketStreamError, Result},
    types::{ChipInfo, StreamAddress},
};

pub type PacketStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>;
pub type PacketSink = Pin<Box<dyn Sink<Bytes, Error = PacketStreamError> + Send + Sync>>;

#[async_trait]
pub trait TransportListener: Send {
    async fn accept(&mut self) -> Result<(PacketStream, PacketSink, ChipInfo, String)>;
    fn local_addr(&self) -> Result<StreamAddress>;
    async fn shutdown(&mut self) -> Result<()>;
}

/// Converts an asynchronous byte stream into framed packet stream and sink.
pub(crate) fn into_packet_stream_sink<S>(stream: S) -> (PacketStream, PacketSink)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
{
    use futures::{SinkExt, StreamExt};
    use tokio_util::codec::{Framed, LengthDelimitedCodec};

    let framed = Framed::new(stream, LengthDelimitedCodec::new());
    let (sink, stream) = framed.split();
    let stream = stream.map(|item| item.map(|b| b.freeze()).map_err(PacketStreamError::Io));
    let sink = sink.sink_map_err(PacketStreamError::Io);
    (Box::pin(stream), Box::pin(sink))
}
