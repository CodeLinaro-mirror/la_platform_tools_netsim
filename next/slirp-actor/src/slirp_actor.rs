use std::fmt;

use libslirp_rs::libslirp::LibSlirp;
use netsim_model::chip::{PacketSink, PacketStream};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::info;

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
    pub(crate) config: libslirp_rs::libslirp_config::SlirpConfig,
    pub(crate) http_proxy: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SlirpStatus {
    pub initialized: bool,
}

impl SlirpActor {
    pub fn new(
        config: libslirp_rs::libslirp_config::SlirpConfig,
        http_proxy: Option<String>,
    ) -> Self {
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
