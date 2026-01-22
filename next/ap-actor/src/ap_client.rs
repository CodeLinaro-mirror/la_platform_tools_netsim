// Copyright 2025-2026 The Android Open Source Project

use super::{ApActor, ApReq};
use actor_framework::ResourceClient;
use netsim_model::client_error::ClientError;

#[derive(Clone, Debug)]
pub struct ApClient {
    client: ResourceClient<ApActor>,
}

impl ApClient {
    pub fn new(client: ResourceClient<ApActor>) -> Self {
        Self { client }
    }

    /// Registers a packet stream and sink for a specific AP.
    pub async fn register(
        &self,
        stream: tokio::sync::mpsc::UnboundedReceiver<bytes::Bytes>,
        sink: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
    ) -> Result<(), ClientError> {
        let _ = self.client.perform_action(None, ApReq::Register { stream, sink }).await.map_err(
            |e| ClientError::Chip(netsim_model::chip_error::ChipError::Internal(e.to_string())),
        )?;
        Ok(())
    }

    pub async fn create_ap(&self, config: super::ApConfig) -> Result<u32, ClientError> {
        self.client.create(config).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    pub async fn destroy_ap(&self, id: u32) -> Result<(), ClientError> {
        self.client.delete(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    pub async fn get_ap(&self, id: u32) -> Result<Option<super::ApState>, ClientError> {
        self.client.get(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    pub async fn list_aps(&self) -> Result<Vec<super::ApState>, ClientError> {
        self.client.list().await.map_err(|e| ClientError::Send(e.to_string()))
    }

    pub async fn update_ap(
        &self,
        id: u32,
        ssid: Option<String>,
        position: Option<netsim_model::device::Position>,
    ) -> Result<super::ApState, ClientError> {
        self.client
            .update(id, crate::ap_actor::ApUpdate { ssid, position })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))
    }
}
