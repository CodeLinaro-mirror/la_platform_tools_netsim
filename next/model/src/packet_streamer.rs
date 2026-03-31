// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::pin::Pin;

use bytes::Bytes;
use futures::Sink;
use tokio_stream::Stream;

// The only error from PacketStream occurs when source closes connection.
/// A stream of packets from the chip.
pub type PacketStream = Box<dyn Stream<Item = Bytes> + Send + Sync + Unpin>;
/// A sink for packets to the chip.
pub type PacketSink = Pin<Box<dyn Sink<Bytes, Error = std::io::Error> + Send + Sync>>;
