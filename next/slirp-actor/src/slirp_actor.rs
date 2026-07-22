// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, SocketAddr},
};

use netsim_model::{PacketSink, PacketStream};
use netsim_packets::MacAddress;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::info;

pub type ClientId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlirpBackend {
    CFfi,
    Native,
}

#[allow(clippy::derivable_impls)]
impl Default for SlirpBackend {
    fn default() -> Self {
        #[cfg(feature = "cuttlefish")]
        {
            SlirpBackend::Native
        }
        #[cfg(not(feature = "cuttlefish"))]
        {
            SlirpBackend::CFfi
        }
    }
}

#[cfg(not(feature = "cuttlefish"))]
pub type SlirpConfig = libslirp_rs::SlirpConfig;

#[cfg(feature = "cuttlefish")]
pub type SlirpConfig = slirp::Config;

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
    SwitchBackend(SlirpBackend),
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
            SlirpReq::SwitchBackend(backend) => {
                write!(f, "SlirpReq::SwitchBackend({:?})", backend)
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

pub(crate) enum SlirpBackendInstance {
    #[cfg(not(feature = "cuttlefish"))]
    CFfi(libslirp_rs::LibSlirp),
    Native {
        uplink_tx: tokio_mpsc::UnboundedSender<bytes::Bytes>,
    },
}

impl SlirpBackendInstance {
    pub fn input(&self, msg: bytes::Bytes) {
        match self {
            #[cfg(not(feature = "cuttlefish"))]
            SlirpBackendInstance::CFfi(slirp) => {
                slirp.input(msg);
            }
            SlirpBackendInstance::Native { uplink_tx } => {
                let _ = uplink_tx.send(msg);
            }
        }
    }

    pub fn shutdown(self) {
        match self {
            #[cfg(not(feature = "cuttlefish"))]
            SlirpBackendInstance::CFfi(slirp) => {
                slirp.shutdown();
            }
            SlirpBackendInstance::Native { .. } => {}
        }
    }
}

pub struct SlirpActor {
    pub(crate) backend_type: SlirpBackend,
    pub(crate) backend_instance: Option<SlirpBackendInstance>,
    pub(crate) config: SlirpConfig,
    pub(crate) http_proxy: Option<String>,
    pub(crate) clients: HashMap<ClientId, ClientInfo>,
    pub(crate) mac_table: HashMap<MacAddress, ClientId>,
    pub(crate) next_stream_id: usize,
    pub(crate) active_stream_id: Option<usize>,
    pub(crate) next_task_id: u32,
    pub(crate) active_task_id: Option<u32>,
}

async fn resolve_dns_servers(host_dns: &str) -> Vec<IpAddr> {
    let mut resolved = Vec::new();
    if host_dns.is_empty() {
        return resolved;
    }
    let futures = host_dns.split(',').map(|addr| {
        let addr = addr.trim().to_string();
        async move {
            if let Ok(ip) = addr.parse::<IpAddr>() {
                return vec![ip];
            }
            if let Ok(socket) = addr.parse::<SocketAddr>() {
                return vec![socket.ip()];
            }
            let host_port = if addr.contains(':') { addr.clone() } else { format!("{}:53", addr) };
            match tokio::net::lookup_host(host_port).await {
                Ok(addrs) => addrs.map(|s| s.ip()).collect::<Vec<_>>(),
                Err(e) => {
                    tracing::warn!("Failed to resolve DNS host '{}': {}", addr, e);
                    vec![]
                }
            }
        }
    });
    let results = futures::future::join_all(futures).await;
    for ip in results.into_iter().flatten() {
        resolved.push(ip);
    }
    resolved
}

#[derive(Clone, Debug)]
pub struct SlirpStatus {
    pub initialized: bool,
}

impl SlirpActor {
    pub async fn new(
        config: SlirpConfig,
        http_proxy: Option<String>,
        host_dns: Option<String>,
    ) -> Self {
        Self::new_with_backend(config, http_proxy, host_dns, SlirpBackend::default()).await
    }

    pub async fn new_with_backend(
        #[allow(unused_mut)] mut config: SlirpConfig,
        http_proxy: Option<String>,
        host_dns: Option<String>,
        backend_type: SlirpBackend,
    ) -> Self {
        let resolved_dns = if let Some(ref host_dns_str) = host_dns {
            resolve_dns_servers(host_dns_str).await
        } else {
            Vec::new()
        };

        if !resolved_dns.is_empty() {
            #[cfg(not(feature = "cuttlefish"))]
            {
                config.host_dns = resolved_dns.iter().map(|ip| SocketAddr::new(*ip, 53)).collect();
            }
            #[cfg(feature = "cuttlefish")]
            {
                config.dns_servers = resolved_dns.clone();
            }
        }

        Self {
            backend_type,
            backend_instance: None,
            config,
            http_proxy,
            clients: HashMap::new(),
            mac_table: HashMap::new(),
            next_stream_id: 1,
            active_stream_id: None,
            next_task_id: 1,
            active_task_id: None,
        }
    }

    pub fn backend(&self) -> SlirpBackend {
        self.backend_type
    }
}

impl Drop for SlirpActor {
    fn drop(&mut self) {
        if let Some(instance) = self.backend_instance.take() {
            info!("Shutting down Slirp backend instance");
            instance.shutdown();
        }
    }
}
