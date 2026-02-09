// Copyright 2025 Google LLC
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
