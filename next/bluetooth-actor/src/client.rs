// Copyright 2025 The Android Open Source Project

//! Bluetooth Client
//!
//! This module provides the [`BluetoothClient`] struct, which is a wrapper around
//! [`ResourceClient<BluetoothActor>`].

use crate::BluetoothAction;
use crate::BluetoothActionResult;
use crate::BluetoothActor;
use actor_framework::ResourceClient;
use netsim_model::chip::{ChipClient, ChipCreate, ChipId};
use netsim_model::client_error::ClientError;

use std::ops::Deref;

/// A client for the Bluetooth actor.
#[derive(Clone, Debug)]
pub struct BluetoothClient(pub ResourceClient<BluetoothActor>);

impl Deref for BluetoothClient {
    type Target = ResourceClient<BluetoothActor>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TODO: Consider generic impl<T> ChipClient for ResourceClient<T>.
#[async_trait::async_trait]
impl ChipClient for BluetoothClient {
    async fn create(&self, params: ChipCreate) -> Result<(), ClientError> {
        self.0.create(params).await.map(|_| ()).map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read(&self, id: ChipId) -> Result<netsim_model::chip::Chip, ClientError> {
        self.0
            .get(id)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?
            .ok_or(ClientError::Chip(netsim_model::chip_error::ChipError::ChipNotFound(id)))
    }

    async fn update(
        &self,
        id: ChipId,
        patch: netsim_model::chip::ChipUpdate,
    ) -> Result<netsim_model::chip::Chip, ClientError> {
        self.0.update(id, patch).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.0.delete(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read_statistics(
        &self,
    ) -> Result<Vec<netsim_model::stats::NetsimRadioStats>, ClientError> {
        // Workaround: GetStatistics is an action, but requires an ID.
        // We list chips first. If empty, return empty stats.
        // If not empty, use the first chip ID to invoke the action (which returns global stats).
        let chips = self.0.list().await.map_err(|e| ClientError::Send(e.to_string()))?;
        if chips.is_empty() {
            return Ok(Vec::new());
        }
        let first_id = chips[0].id;
        match self.0.perform_action(Some(ChipId(first_id)), BluetoothAction::GetStatistics).await {
            Ok(BluetoothActionResult::Statistics(stats)) => Ok(stats),
            Ok(_) => Err(ClientError::Recv("Unexpected action result".into())),
            Err(e) => Err(ClientError::Send(e.to_string())),
        }
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.0.list().await.map(|chips| chips.len()).map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        // ResourceClient does not support explicit shutdown.
        // Dropping the client will eventually shut down the actor if it's the last one.
        Ok(())
    }

    async fn reset(&self, id: ChipId) -> Result<(), ClientError> {
        self.0
            .perform_action(Some(id), BluetoothAction::Reset { id })
            .await
            .map(|_| ())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
