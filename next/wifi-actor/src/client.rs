// Copyright 2025 The Android Open Source Project

use actor_framework::ResourceClient;
use netsim_model::{
    chip::{Chip, ChipClient, ChipCreate, ChipId, ChipUpdate, ChipVariantUpdate},
    chip_error::ChipError,
    client_error::ClientError,
    stats::NetsimRadioStats,
};

use crate::wifi_actor::WifiActor;

#[derive(Clone, Debug)]
pub struct WifiClient(ResourceClient<WifiActor>);

impl WifiClient {
    pub fn new(client: ResourceClient<WifiActor>) -> Self {
        Self(client)
    }

    pub fn debug_trait_check(&self) {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Self>();
    }

    pub async fn set_rf_state(&self, id: ChipId, enabled: bool) -> Result<(), ClientError> {
        let patch = ChipUpdate {
            variant: Some(ChipVariantUpdate::Wifi(Default::default())),
            enabled: Some(enabled),
            ..Default::default()
        };
        self.0.update(id, patch).await.map(|_| ()).map_err(|e| ClientError::Send(e.to_string()))
    }
}

#[async_trait::async_trait]
impl ChipClient for WifiClient {
    async fn create(&self, params: ChipCreate) -> Result<(), ClientError> {
        self.0.create(params).await.map(|_| ()).map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        self.0
            .get(id)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?
            .ok_or(ClientError::Chip(ChipError::ChipNotFound(id)))
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.0.update(id, patch).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.0.delete(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError> {
        match self.0.perform_action(None, crate::wifi_actor::WifiReq::GetStatistics).await {
            Ok(crate::wifi_actor::WifiResponse::Statistics(stats)) => Ok(stats),
            Ok(_) => Err(ClientError::Recv("Unexpected action result".into())),
            Err(e) => Err(ClientError::Send(e.to_string())),
        }
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.0.list().await.map(|chips| chips.len()).map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        self.0.shutdown().await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn reset(&self, id: ChipId) -> Result<(), ClientError> {
        self.0
            .perform_action(Some(id), crate::wifi_actor::WifiReq::Reset { id })
            .await
            .map(|_| ())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
