// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, fmt};

#[rustfmt::skip]
use libslirp_rs::{LibSlirp, SlirpConfig, lookup_host_dns};
use netsim_model::{PacketSink, PacketStream};
use netsim_packets::MacAddress;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{info, warn};

pub type ClientId = u32;

pub enum SlirpReq {
    SendPacket(bytes::Bytes),
    Register {
        client_id: ClientId,
        stream: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Sync + Send>>,
        sink: netsim_model::PacketSink,
        notifier: Option<tokio_mpsc::UnboundedSender<netsim_model::ChipId>>,
    },
    Unregister {
        client_id: ClientId,
    },
}

impl std::fmt::Debug for SlirpReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlirpReq::SendPacket(_) => write!(f, "SlirpReq::SendPacket(...)"),
            SlirpReq::Register { client_id, .. } => {
                write!(f, "SlirpReq::Register {{ client_id: {client_id}, ... }}")
            }
            SlirpReq::Unregister { client_id } => {
                write!(f, "SlirpReq::Unregister {{ client_id: {client_id} }}")
            }
        }
    }
}

pub struct SlirpCreate {
    pub packet_stream: Option<PacketStream>,
    pub packet_sink: Option<PacketSink>,
}

impl fmt::Debug for SlirpCreate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlirpCreate")
            .field("packet_stream", &self.packet_stream.is_some())
            .field("packet_sink", &self.packet_sink.is_some())
            .finish()
    }
}

pub struct ClientInfo {
    pub sink: tokio_mpsc::UnboundedSender<bytes::Bytes>,
    pub notifier: Option<tokio_mpsc::UnboundedSender<netsim_model::ChipId>>,
}

pub struct SlirpActor {
    pub(crate) libslirp: Option<LibSlirp>,
    pub(crate) config: SlirpConfig,
    pub(crate) http_proxy: Option<String>,
    pub(crate) clients: HashMap<ClientId, ClientInfo>,
    pub(crate) mac_table: HashMap<MacAddress, ClientId>,
}

#[derive(Clone, Debug)]
pub struct SlirpStatus {
    pub initialized: bool,
}

impl SlirpActor {
    pub async fn new(
        mut config: SlirpConfig,
        http_proxy: Option<String>,
        host_dns: Option<String>,
    ) -> Self {
        if let Some(host_dns_str) = host_dns {
            match lookup_host_dns(&host_dns_str).await {
                Ok(addrs) => config.host_dns = addrs,
                Err(e) => warn!("Failed to resolve host-dns '{}': {}", host_dns_str, e),
            }
        }
        Self {
            libslirp: None,
            config,
            http_proxy,
            clients: HashMap::new(),
            mac_table: HashMap::new(),
        }
    }
}

impl Drop for SlirpActor {
    fn drop(&mut self) {
        if let Some(slirp) = self.libslirp.take() {
            info!("Shutting down LibSlirp");
            slirp.shutdown();
        }
    }
}
