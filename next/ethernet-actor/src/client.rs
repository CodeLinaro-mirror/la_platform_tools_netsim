// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::ResourceClient;
use netsim_model::{
    Chip, ChipClient, ChipCreate, ChipError, ChipId, ChipUpdate, ClientError, NetsimRadioStats,
};

use crate::ethernet_actor::{EthernetActor, EthernetReq, EthernetResponse};

#[derive(Clone)]
pub struct EthernetClient {
    pub(crate) inner: Box<dyn actor_framework::ActorClient<EthernetActor>>,
}

impl std::fmt::Debug for EthernetClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EthernetClient").finish_non_exhaustive()
    }
}

impl EthernetClient {
    pub fn new(client: ResourceClient<EthernetActor>) -> Self {
        Self { inner: Box::new(client) }
    }
}

#[async_trait::async_trait]
impl ChipClient for EthernetClient {
    async fn create(&self, id: ChipId, params: ChipCreate) -> Result<(), ClientError> {
        self.inner
            .create_with_id(id, params)
            .await
            .map(|_| ())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        self.inner
            .get(id)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?
            .ok_or(ClientError::Chip(ChipError::ChipNotFound(id)))
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.inner.update(id, patch).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.inner.delete(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError> {
        match self.inner.perform_action(None, EthernetReq::GetStatistics).await {
            Ok(EthernetResponse::Statistics(stats)) => Ok(stats),
            Ok(_) => Err(ClientError::Recv("Unexpected action result".into())),
            Err(e) => Err(ClientError::Send(e.to_string())),
        }
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.inner
            .list()
            .await
            .map(|chips| chips.len())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        self.inner.shutdown().await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn reset(&self, id: ChipId) -> Result<Chip, ClientError> {
        match self.inner.perform_action(Some(id), EthernetReq::Reset { id }).await {
            Ok(EthernetResponse::Chip(chip)) => Ok(chip),
            Ok(_) => Err(ClientError::Recv("Unexpected action result for reset".into())),
            Err(e) => Err(ClientError::Send(e.to_string())),
        }
    }

    async fn get_global_stats(&self) -> Result<Option<Vec<u8>>, ClientError> {
        Ok(None)
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
