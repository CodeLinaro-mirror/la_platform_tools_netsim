// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::VecDeque,
    fs::File as StdFile,
    os::unix::io::OwnedFd,
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use command_fds::inherited::take_fd_ownership;
use futures::{stream::StreamExt, SinkExt};
use netsim_model::initial_info::{Chip, ChipInfo, ChipKind, DeviceInfo};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncWrite, ReadBuf},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::{
    error::{PacketStreamError, Result},
    transport::traits::{PacketSink, PacketStream, TransportListener},
    types::StreamAddress,
};

struct DualFd {
    reader: File,
    writer: File,
}

impl AsyncRead for DualFd {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for DualFd {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub serial: String,
    pub chips: Vec<ChipConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipConfig {
    pub kind: String,
    #[serde(rename = "fdIn")]
    pub fd_in: i32,
    #[serde(rename = "fdOut")]
    pub fd_out: Option<i32>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualFdConfig {
    pub devices: Vec<DeviceConfig>,
}

pub struct DualFdListener {
    config: DualFdConfig,
    pending_streams: VecDeque<(String, String, (OwnedFd, OwnedFd))>,
}

impl DualFdListener {
    pub async fn new(config: DualFdConfig) -> Result<Self> {
        #[allow(unused_mut)]
        let mut listener = Self { config, pending_streams: VecDeque::new() };
        listener.prepare_streams()?;
        Ok(listener)
    }

    fn prepare_streams(&mut self) -> Result<()> {
        self.pending_streams.clear();
        for device in &self.config.devices {
            for chip in &device.chips {
                let in_fd = take_fd_ownership(chip.fd_in).map_err(|e| {
                    PacketStreamError::Socket(crate::error::SocketError::AcceptFailed(
                        e.to_string(),
                    ))
                })?;
                let out_fd = chip.fd_out.ok_or_else(|| {
                    PacketStreamError::InvalidConfig(
                        "DualFdStream requires an output file descriptor.".to_string(),
                    )
                })?;
                let out_fd = take_fd_ownership(out_fd).map_err(|e| {
                    PacketStreamError::Socket(crate::error::SocketError::AcceptFailed(
                        e.to_string(),
                    ))
                })?;
                self.pending_streams.push_back((
                    device.serial.clone(),
                    chip.kind.clone(),
                    (in_fd, out_fd),
                ));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl TransportListener for DualFdListener {
    async fn accept(&mut self) -> Result<(PacketStream, PacketSink, ChipInfo, String)> {
        match self.pending_streams.pop_front() {
            Some((device_serial, chip_kind, (in_fd, out_fd))) => {
                let guid = format!("dualfd-{}-{}", device_serial, chip_kind);
                let chip_info = ChipInfo {
                    device_info: Some(DeviceInfo {
                        name: device_serial.clone(),
                        id: device_serial,
                    }),
                    chip: Some(Chip {
                        name: chip_kind.clone(),
                        kind: ChipKind::Unspecified,
                        id: chip_kind,
                        manufacturer: "".to_string(),
                        product_name: "".to_string(),
                    }),
                    name: String::new(),
                };

                let dual_fd = DualFd {
                    reader: File::from_std(StdFile::from(in_fd)),
                    writer: File::from_std(StdFile::from(out_fd)),
                };

                let framed = Framed::new(dual_fd, LengthDelimitedCodec::new());
                let (sink, stream) = framed.split();

                let stream =
                    stream.map(|item| item.map(|b| b.freeze()).map_err(PacketStreamError::Io));
                let sink = sink.sink_map_err(PacketStreamError::Io);

                Ok((Box::pin(stream), Box::pin(sink), chip_info, guid))
            }
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> Result<StreamAddress> {
        let total_chips: usize = self.config.devices.iter().map(|d| d.chips.len()).sum();
        Ok(StreamAddress::Fd {
            in_fd: total_chips as i32,
            out_fd: Some(self.config.devices.len() as i32),
        })
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.pending_streams.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
