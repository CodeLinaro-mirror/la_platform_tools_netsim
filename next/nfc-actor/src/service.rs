// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorService, DynContext};
use futures::{SinkExt, StreamExt};
use netsim_model::{ChipCreate, ChipError, ChipId, ChipRequest, ChipUpdate};
use tracing::{error, info};

use crate::{
    error::NfcError,
    nfc_actor::{ChipState, NfcActor},
};

impl ActorService for NfcActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = ChipRequest;
    type ActionResult = ();
    type Error = NfcError;
    type Entity = netsim_model::Chip;
    type TypedStream = (); // No typed streams for now

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        mut params: Self::Create,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = id.ok_or_else(|| ChipError::InvalidArguments("missing chip id".into()))?;
        let device_id = params.chip.device_id;

        if self.active_chips.contains_key(&chip_id) {
            return Err(NfcError::Chip(ChipError::ChipExists(chip_id.0)));
        }

        let packet_stream = params.packet_stream.take().ok_or(NfcError::MissingStreamSink)?;
        let mut packet_sink = params.packet_sink.take().ok_or(NfcError::MissingStreamSink)?;

        let scene_client = self
            .scene_client
            .as_ref()
            .ok_or_else(|| {
                NfcError::IoError(std::io::Error::other("Scene client not initialized"))
            })?
            .clone();

        // Create duplex stream for Casimir bridge
        let (nfc_io, casimir_io) = tokio::io::duplex(1024);

        let (casimir_rx, casimir_tx) = tokio::io::split(casimir_io);
        let casimir_device_id = scene_client
            .add_device(move |id, rf_tx| casimir::Device::nci(id, casimir_rx, casimir_tx, rf_tx))
            .await
            .map_err(|e| NfcError::IoError(std::io::Error::other(e)))?;

        info!("Added NFC device to Casimir scene with ID {}", casimir_device_id);

        // Bridge Casimir -> Guest
        let (nfc_reader, nfc_writer) = tokio::io::split(nfc_io);
        let mut stream = tokio_util::codec::length_delimited::Builder::new()
            .length_field_offset(2)
            .length_field_length(1)
            .num_skip(0)
            .new_read(nfc_reader);
        let chip_id_clone = chip_id;
        let task_1 = Box::pin(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes_mut) => {
                        let bytes = bytes_mut.freeze();
                        if let Err(e) = packet_sink.send(bytes).await {
                            error!("Failed to send packet to guest: {:?}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("NFC reader error: {:?}", e);
                        break;
                    }
                }
            }
            info!("NFC forward loop finished");
            chip_id_clone
        });
        ctx.spawn(chip_id, task_1);

        // Bridge Guest -> Casimir
        ctx.add_stream(chip_id, Box::pin(packet_stream));

        self.active_chips
            .insert(chip_id, ChipState { device_id, enabled: true, casimir_device_id, nfc_writer });

        info!("NFC chip {} created for device {}", chip_id, device_id);

        Ok(chip_id)
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        if let Some(state) = self.active_chips.remove(&id) {
            info!("Deleting NFC chip {}", id);
            // Notify DeviceClient
            let _ = self.device_client.notify_chip_removed(state.device_id, id).await;

            // Abort the Casimir -> Guest task
            ctx.abort(id);

            // Remove the guest stream
            ctx.remove_stream(id);

            // Remove device from Casimir scene
            if let Some(ref scene_client) = self.scene_client
                && let Err(e) = scene_client.remove_device(state.casimir_device_id).await
            {
                error!(
                    "Failed to remove device {} from Casimir scene: {:?}",
                    state.casimir_device_id, e
                );
            }
        }
        Ok(())
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        if let Some(state) = self.active_chips.get(&id) {
            Ok(Some(netsim_model::Chip {
                kind: netsim_model::ChipKind::NFC,
                id: id.0,
                name: format!("nfc-{}", id.0),
                device_id: state.device_id,
                enabled: state.enabled,
                variant: Some(netsim_model::ChipVariant::Nfc(netsim_model::Nfc {
                    radio: netsim_model::Radio { state: Some(state.enabled), ..Default::default() },
                })),
                ..Default::default()
            }))
        } else {
            Ok(None)
        }
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        let state = self
            .active_chips
            .get_mut(&id)
            .ok_or_else(|| NfcError::Chip(ChipError::ChipNotFound(id)))?;

        if let Some(enabled) = update.enabled {
            state.enabled = enabled;
        }
        if let Some(netsim_model::ChipVariantUpdate::Nfc(netsim_model::NfcUpdate {
            radio: netsim_model::RadioUpdate { state: Some(enabled), .. },
        })) = &update.variant
        {
            state.enabled = *enabled;
        }

        self.handle_get(id, ctx).await?.ok_or_else(|| NfcError::Chip(ChipError::ChipNotFound(id)))
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        _action: Self::Action,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        Ok(())
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        let mut chips = Vec::new();
        for (id, state) in &self.active_chips {
            chips.push(netsim_model::Chip {
                kind: netsim_model::ChipKind::NFC,
                id: id.0,
                name: format!("nfc-{}", id.0),
                device_id: state.device_id,
                enabled: state.enabled,
                variant: Some(netsim_model::ChipVariant::Nfc(netsim_model::Nfc {
                    radio: netsim_model::Radio { state: Some(state.enabled), ..Default::default() },
                })),
                ..Default::default()
            });
        }
        Ok(chips)
    }
}
