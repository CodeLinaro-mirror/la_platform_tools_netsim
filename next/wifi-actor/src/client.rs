// Copyright 2025 The Android Open Source Project

use crate::wifi_actor::WifiActor;
use actor_framework::ResourceClient;
use netsim_model::client_error::ClientError;

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

    pub async fn set_rf_state(
        &self,
        id: netsim_model::chip::ChipId,
        enabled: bool,
    ) -> Result<(), ClientError> {
        use netsim_model::chip::{ChipUpdate, ChipVariantUpdate};
        let patch = ChipUpdate {
            variant: Some(ChipVariantUpdate::Wifi(Default::default())),
            enabled: Some(enabled),
            ..Default::default()
        };
        self.0.update(id, patch).await.map(|_| ()).map_err(|e| ClientError::Send(e.to_string()))
    }
}

#[async_trait::async_trait]
impl netsim_model::chip::ChipClient for WifiClient {
    async fn create(&self, params: netsim_model::chip::ChipCreate) -> Result<(), ClientError> {
        self.0.create(params).await.map(|_| ()).map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read(
        &self,
        id: netsim_model::chip::ChipId,
    ) -> Result<netsim_model::chip::Chip, ClientError> {
        self.0
            .get(id)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?
            .ok_or(ClientError::Chip(netsim_model::chip_error::ChipError::ChipNotFound(id)))
    }

    async fn update(
        &self,
        id: netsim_model::chip::ChipId,
        patch: netsim_model::chip::ChipUpdate,
    ) -> Result<netsim_model::chip::Chip, ClientError> {
        self.0.update(id, patch).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn delete(&self, id: netsim_model::chip::ChipId) -> Result<(), ClientError> {
        self.0.delete(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read_statistics(
        &self,
    ) -> Result<Box<[netsim_model::stats::NetsimRadioStats]>, ClientError> {
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
        Ok(())
    }

    async fn reset(&self, id: netsim_model::chip::ChipId) -> Result<(), ClientError> {
        self.0
            .perform_action(Some(id), crate::wifi_actor::WifiReq::Reset { id })
            .await
            .map(|_| ())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    fn clone_box(&self) -> Box<dyn netsim_model::chip::ChipClient> {
        Box::new(self.clone())
    }
}
