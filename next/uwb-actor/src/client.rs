// Copyright 2026 The Android Open Source Project

use actor_framework::{FrameworkError, ResourceClient};
use async_trait::async_trait;
use netsim_model::{
    chip::{Chip, ChipClient, ChipCreate, ChipId, ChipUpdate},
    client_error::ClientError,
    stats::NetsimRadioStats,
};

use crate::uwb_actor::UwbActor;

// Helper to handle downcast failure gracefully if we wanted to preserve error
// message But downcast consuming only on success is fine if we just convert to
// string on failure... Actually downcast failure returns the box.
fn map_framework_error_smart(e: FrameworkError) -> ClientError {
    match e {
        FrameworkError::ServiceError(boxed) => {
            match boxed.downcast::<netsim_model::chip_error::ChipError>() {
                Ok(chip_error) => ClientError::Chip(*chip_error),
                Err(boxed) => ClientError::Send(boxed.to_string()),
            }
        }
        _ => ClientError::Send(e.to_string()),
    }
}

/// A client for communicating with the UWB Actor.
/// Wraps a generic `ResourceClient` and implements `ChipClient`.
#[derive(Clone, Debug)]
pub struct UwbClient(pub ResourceClient<UwbActor>);

#[async_trait]
impl ChipClient for UwbClient {
    async fn create(&self, params: ChipCreate) -> Result<(), ClientError> {
        self.0.create(params).await.map(|_| ()).map_err(map_framework_error_smart)
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        self.0
            .get(id)
            .await
            .map_err(map_framework_error_smart)?
            .ok_or(ClientError::Chip(netsim_model::chip_error::ChipError::ChipNotFound(id)))
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.0.update(id, patch).await.map_err(map_framework_error_smart)
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.0.delete(id).await.map_err(map_framework_error_smart)
    }

    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError> {
        match self.0.perform_action(None, crate::UwbAction::GetStatistics).await {
            Ok(crate::UwbActionResult::Statistics(stats)) => Ok(stats),
            Ok(_) => Err(ClientError::Recv("Unexpected action result".into())),
            Err(e) => Err(map_framework_error_smart(e)),
        }
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.0.list().await.map(|chips| chips.len()).map_err(map_framework_error_smart)
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        self.0.shutdown().await.map_err(map_framework_error_smart)
    }

    async fn reset(&self, id: ChipId) -> Result<(), ClientError> {
        self.0
            .perform_action(Some(id), crate::UwbAction::Reset { id })
            .await
            .map(|_| ())
            .map_err(map_framework_error_smart)
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
