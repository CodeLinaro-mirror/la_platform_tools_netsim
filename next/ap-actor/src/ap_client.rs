// Copyright 2025-2026 The Android Open Source Project

use super::{ApActor, ApReq};
use actor_framework::ResourceClient;
use netsim_model::client_error::ClientError;

#[cfg_attr(feature = "testing", mockall::automock)]
#[async_trait::async_trait]
pub trait ApClientTrait: core::fmt::Debug + Send + Sync {
    async fn register(
        &self,
        stream: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Send>>,
        sink: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
        shared_keys: std::sync::Arc<super::shared::SharedKeyStore>,
        beacon_interval: std::time::Duration,
    ) -> Result<(), ClientError>;

    async fn create_ap(&self, config: super::ApConfig) -> Result<u32, ClientError>;

    async fn destroy_ap(&self, id: u32) -> Result<(), ClientError>;

    async fn get_ap(&self, id: u32) -> Result<Option<super::ApState>, ClientError>;

    async fn list_aps(&self) -> Result<Vec<super::ApState>, ClientError>;

    async fn update_ap(
        &self,
        id: u32,
        ssid: Option<String>,
        position: Option<netsim_model::device::Position>,
    ) -> Result<super::ApState, ClientError>;
}

#[derive(Clone, Debug)]
pub struct ApClient {
    client: ResourceClient<ApActor>,
}

impl ApClient {
    pub fn new(client: ResourceClient<ApActor>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl ApClientTrait for ApClient {
    async fn register(
        &self,
        stream: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Send>>,
        sink: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
        shared_keys: std::sync::Arc<super::shared::SharedKeyStore>,
        beacon_interval: std::time::Duration,
    ) -> Result<(), ClientError> {
        let _ = self
            .client
            .perform_action(None, ApReq::Register { stream, sink, shared_keys, beacon_interval })
            .await
            .map_err(|e| {
                ClientError::Chip(netsim_model::chip_error::ChipError::Internal(e.to_string()))
            })?;
        Ok(())
    }

    async fn create_ap(&self, config: super::ApConfig) -> Result<u32, ClientError> {
        self.client.create(config).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn destroy_ap(&self, id: u32) -> Result<(), ClientError> {
        self.client.delete(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn get_ap(&self, id: u32) -> Result<Option<super::ApState>, ClientError> {
        self.client.get(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn list_aps(&self) -> Result<Vec<super::ApState>, ClientError> {
        self.client.list().await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn update_ap(
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
