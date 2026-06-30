// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::VecDeque, fs::File as StdFile, os::unix::io::OwnedFd, str::FromStr};

use async_trait::async_trait;
use command_fds::inherited::take_fd_ownership;
use futures::{SinkExt, stream::StreamExt};
use netsim_types::{Chip, ChipInfo, ChipKind, DeviceInfo};
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncWriteExt,
    net::unix::pipe::{Receiver, Sender},
};
use tokio_util::codec::FramedRead;

use crate::{
    error::{PacketStreamError, Result},
    transport::{
        H4Codec, ModemCodec, NciCodec, UciCodec,
        traits::{PacketSink, PacketStream, TransportListener},
    },
    types::StreamAddress,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub name: String,
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
    #[serde(rename = "simType")]
    pub sim_type: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualFdConfig {
    pub devices: Vec<DeviceConfig>,
}

pub struct DualFdListener {
    config: DualFdConfig,
    pending_streams: VecDeque<(String, String, Option<i32>, (OwnedFd, OwnedFd))>,
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
                    PacketStreamError::InvalidConfig(format!(
                        "Failed to claim input FD {}: {e}",
                        chip.fd_in
                    ))
                })?;

                let out_fd = match chip.fd_out {
                    Some(fd) if fd != chip.fd_in => take_fd_ownership(fd).map_err(|e| {
                        PacketStreamError::InvalidConfig(format!(
                            "Failed to claim output FD {fd}: {e}"
                        ))
                    })?,
                    _ => in_fd.try_clone().map_err(|e| {
                        PacketStreamError::InvalidConfig(format!(
                            "Failed to clone bidirectional FD {}: {e}",
                            chip.fd_in
                        ))
                    })?,
                };

                self.pending_streams.push_back((
                    device.name.clone(),
                    chip.kind.clone(),
                    chip.sim_type,
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
        while let Some((device_name, chip_kind, sim_type, (in_fd, out_fd))) =
            self.pending_streams.pop_front()
        {
            let guid = format!("dualfd-{}", device_name);
            let kind = ChipKind::from_str(&chip_kind).unwrap_or(ChipKind::UNSPECIFIED);

            let chip_info = ChipInfo {
                device_info: Some(DeviceInfo {
                    name: device_name.clone(),
                    id: device_name,
                    ..Default::default()
                }),
                chip: Some(Chip {
                    name: chip_kind.clone(),
                    kind,
                    id: chip_kind.clone(),
                    manufacturer: "".to_string(),
                    product_name: "".to_string(),
                    address: "".to_string(),
                    sim_type,
                }),
                name: String::new(),
            };

            let reader = Receiver::from_owned_fd(out_fd)?;
            let writer = Sender::from_owned_fd(in_fd)?;

            let stream: PacketStream = match kind {
                ChipKind::BLUETOOTH => {
                    let framed = FramedRead::new(reader, H4Codec);
                    Box::pin(framed.map(|item| item.map_err(PacketStreamError::Io)))
                }
                ChipKind::UWB => {
                    let framed = FramedRead::new(reader, UciCodec);
                    Box::pin(framed.map(|item| item.map_err(PacketStreamError::Io)))
                }
                ChipKind::NFC => {
                    let framed = FramedRead::new(reader, NciCodec);
                    Box::pin(framed.map(|item| item.map_err(PacketStreamError::Io)))
                }
                ChipKind::CELLULAR => {
                    let framed = FramedRead::new(reader, ModemCodec);
                    Box::pin(framed.map(|item| item.map_err(PacketStreamError::Io)))
                }

                _ => {
                    tracing::warn!(
                        "Unsupported chip kind for FD transport: {:?}, skipping",
                        chip_kind
                    );
                    continue;
                }
            };

            let sink = futures::sink::unfold(writer, |mut writer, item: bytes::Bytes| async move {
                writer.write_all(&item).await?;
                Ok(writer)
            });
            let sink: PacketSink = Box::pin(sink.sink_map_err(PacketStreamError::Io));

            return Ok((stream, sink, chip_info, guid));
        }
        std::future::pending().await
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
