// Copyright 2025 The Android Open Source Project

use actor_framework::{FrameworkError, ResourceClient};
use async_trait::async_trait;
use netsim_model::{
    chip::{Chip, ChipClient, ChipCreate, ChipId, ChipUpdate},
    client_error::ClientError,
    stats::NetsimRadioStats,
};

use crate::{CellActor, CellError};

fn map_framework_error(e: FrameworkError) -> ClientError {
    match e {
        FrameworkError::ServiceError(boxed) => {
            // Try to downcast to CellError first
            if let Some(cell_error) = boxed.downcast_ref::<CellError>() {
                return ClientError::Chip(netsim_model::chip_error::ChipError::Internal(
                    cell_error.to_string(),
                ));
            }
            ClientError::Send(boxed.to_string())
        }
        _ => ClientError::Send(e.to_string()),
    }
}

#[derive(Clone, Debug)]
pub struct CellClient(pub ResourceClient<CellActor>);

#[async_trait]
impl ChipClient for CellClient {
    async fn create(&self, params: ChipCreate) -> Result<(), ClientError> {
        self.0.create(params).await.map(|_| ()).map_err(map_framework_error)
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        self.0
            .get(id)
            .await
            .map_err(map_framework_error)?
            .ok_or(ClientError::Chip(netsim_model::chip_error::ChipError::ChipNotFound(id)))
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.0.update(id, patch).await.map_err(map_framework_error)
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.0.delete(id).await.map_err(map_framework_error)
    }

    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError> {
        // Cell doesn't implement statistics yet
        Ok(Box::new([]))
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.0.list().await.map(|chips| chips.len()).map_err(map_framework_error)
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        self.0.shutdown().await.map_err(map_framework_error)
    }

    async fn reset(&self, _id: ChipId) -> Result<(), ClientError> {
        // Not implemented
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
