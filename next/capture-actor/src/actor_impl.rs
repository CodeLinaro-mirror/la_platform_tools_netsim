//! # Capture Actor Implementation
//!
//! This module implements the `ActorEntity` trait for `CaptureEntity`.
//! It handles the lifecycle of the capture entity, including creation,
//! updates (enabling/disabling capture), and deletion.
//! It also delegates action handling to the `handlers` module.

use crate::bt_pcap::BluetoothH4Writer;
use crate::context::CaptureContext;
use crate::entity::CaptureEntity;
use crate::error::CaptureError;
use crate::writer::CaptureWriter;
use actor_framework::{ActorEntity, Runtime};
use async_trait::async_trait;
use capture_api::{CaptureAction, CaptureActionResult, CaptureCreate, CaptureInfo};
use netsim_model::chip::{ChipId, ChipKind};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

#[async_trait]
impl ActorEntity for CaptureEntity {
    type Id = ChipId;
    type Create = CaptureCreate;
    type Update = bool; // Enabled status
    type Action = CaptureAction;
    type ActionResult = CaptureActionResult;
    type Context = CaptureContext;
    type Error = CaptureError;
    type ListResponse = Vec<CaptureInfo>;

    fn from_create_params(id: Self::Id, params: Self::Create) -> Result<Self, Self::Error> {
        Ok(Self {
            chip_id: id,
            chip_kind: params.chip_kind,
            device_name: params.device_name,
            enabled: params.default_enabled, // Still allow per-chip override in create params
            enabled_flag: params.enabled_flag,
        })
    }

    async fn on_create(
        &mut self,
        context: &mut Self::Context,
        runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        // Register the enabled flag in the context so streams can access it.
        {
            context.flags.insert(self.chip_id, self.enabled_flag.clone());
        }

        // If enabled by default (either via params or context), set up the writer
        let default_enabled = self.enabled || context.default_capture_enabled;
        if default_enabled {
            self.enabled = true;
            self.enabled_flag.store(true, Ordering::SeqCst);
            self.on_update(true, context, runtime).await?;
        }
        Ok(())
    }

    async fn on_update(
        &mut self,
        update: Self::Update,
        context: &mut Self::Context,
        runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        if self.enabled == update {
            return Ok(());
        }
        self.enabled = update; // Update the serializable field
        self.enabled_flag.store(update, Ordering::SeqCst); // Update the atomic flag

        if self.enabled {
            if !context.writers.contains_key(&self.chip_id) {
                // Create writer when capture is first enabled
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let filename =
                    format!("capture_{}_{}_{}.pcap", self.device_name, self.chip_id.0, timestamp);
                let filepath = if let Some(dir) = context.capture_dir.as_ref() {
                    dir.join(&filename)
                } else {
                    PathBuf::from(&filename)
                };
                let writer: Box<dyn CaptureWriter> = match self.chip_kind {
                    ChipKind::BLUETOOTH | ChipKind::BleBeacon => {
                        Box::new(BluetoothH4Writer::new(&filepath)?)
                    }
                    _ => {
                        // Fallback or other kinds
                        // For now, just use BluetoothH4Writer as default for now, or handle other types.
                        Box::new(BluetoothH4Writer::new(&filepath)?)
                    }
                };
                context.writers.insert(self.chip_id, writer);
            }
        } else {
            // Disable capture: remove the writer to close the file
            context.writers.remove(&self.chip_id);
        }
        Ok(())
    }

    async fn on_delete(
        &self,
        context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        // Clean up resources when the entity is deleted.
        context.writers.remove(&self.chip_id);
        context.flags.remove(&self.chip_id);
        Ok(())
    }

    async fn handle_action(
        &mut self,
        action: Self::Action,
        context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<Self::ActionResult, Self::Error> {
        crate::handlers::handle_action(self, action, context, _runtime).await
    }

    fn on_list(
        entities: &std::collections::HashMap<Self::Id, Self>,
        context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Self::ListResponse {
        crate::handlers::on_list(entities, context)
    }
}
