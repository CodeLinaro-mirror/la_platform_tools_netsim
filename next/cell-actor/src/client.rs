// Copyright 2025 The Android Open Source Project

use actor_framework::ResourceClient;
use async_trait::async_trait;
use futures::TryFutureExt;
use netsim_model::{
    chip::{Chip, ChipClient, ChipCreate, ChipId, ChipUpdate},
    client_error::ClientError,
    stats::NetsimRadioStats,
};

use crate::CellActor;

#[derive(Clone, Debug)]
pub struct CellClient(pub ResourceClient<CellActor>);

#[async_trait]
impl ChipClient for CellClient {
    async fn create(&self, id: ChipId, params: ChipCreate) -> Result<(), ClientError> {
        self.0.create_with_id(id, params).err_into::<ClientError>().await.map(|_| ())
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        self.0
            .get(id)
            .err_into::<ClientError>()
            .await?
            .ok_or(ClientError::Chip(netsim_model::chip_error::ChipError::ChipNotFound(id)))
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.0.update(id, patch).err_into::<ClientError>().await
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.0.delete(id).err_into::<ClientError>().await
    }

    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError> {
        // Cell doesn't implement statistics yet
        Ok(Box::new([]))
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.0.list().err_into::<ClientError>().await.map(|chips| chips.len())
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        self.0.shutdown().err_into::<ClientError>().await
    }

    async fn reset(&self, id: ChipId) -> Result<Chip, ClientError> {
        // Not implemented
        self.read(id).await
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
