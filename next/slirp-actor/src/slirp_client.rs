use actor_framework::ResourceClient;
use netsim_model::{
    chip::{PacketSink, PacketStream},
    client_error::ClientError,
};

use crate::slirp_actor::{SlirpActor, SlirpReq};

#[derive(Clone, Debug)]
pub struct SlirpClient {
    client: ResourceClient<SlirpActor>,
}

impl SlirpClient {
    pub fn new(client: ResourceClient<SlirpActor>) -> Self {
        Self { client }
    }

    pub async fn create(
        &self,
        packet_stream: Option<PacketStream>,
        packet_sink: Option<PacketSink>,
    ) -> Result<(), ClientError> {
        self.client
            .create(super::SlirpCreate { packet_stream, packet_sink })
            .await
            .map(|_| ())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    pub async fn send_packet(&self, packet: bytes::Bytes) {
        let _ = self.client.perform_action(None, SlirpReq::SendPacket(packet)).await;
    }

    pub async fn register(
        &self,
        stream: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Send>>,
        sink: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
    ) -> Result<(), ClientError> {
        self.client
            .perform_action(None, SlirpReq::Register { stream, sink })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        Ok(())
    }
}
