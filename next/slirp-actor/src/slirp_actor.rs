use std::fmt;

use libslirp_rs::{
    libslirp::LibSlirp,
    libslirp_config::{lookup_host_dns, SlirpConfig},
};
use netsim_model::chip::{PacketSink, PacketStream};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{info, warn};

pub enum SlirpReq {
    SendPacket(bytes::Bytes),
    Register {
        stream: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Send>>,
        sink: tokio_mpsc::UnboundedSender<bytes::Bytes>,
    },
}

impl std::fmt::Debug for SlirpReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlirpReq::SendPacket(_) => write!(f, "SlirpReq::SendPacket(...)"),
            SlirpReq::Register { .. } => write!(f, "SlirpReq::Register {{ ... }}"),
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

pub struct SlirpActor {
    pub(crate) libslirp: Option<LibSlirp>,
    pub(crate) config: SlirpConfig,
    pub(crate) http_proxy: Option<String>,
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
        Self { libslirp: None, config, http_proxy }
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
