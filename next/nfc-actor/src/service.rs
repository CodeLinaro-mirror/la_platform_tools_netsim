// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorService, DynContext};
use futures::{SinkExt, StreamExt};
use netsim_model::{ChipCreate, ChipError, ChipId, ChipUpdate};
use tracing::{error, info};

use crate::{
    error::NfcError,
    nfc_actor::{ChipState, NfcAction, NfcActor},
};

impl ActorService for NfcActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = NfcAction;
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
            .length_adjustment(3)
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
        action: Self::Action,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            NfcAction::Generic(_req) => Ok(()),
            NfcAction::CreateControlChannel { respond_to } => {
                info!("NfcActor: Received CreateControlChannel action");
                let (grpc_io, casimir_io) = tokio::io::duplex(1024);
                if let Some(ref scene_client) = self.scene_client {
                    info!("NfcActor: Calling scene_client.add_device...");
                    let add_result = scene_client
                        .add_device(move |id, rf_tx| {
                            let (rx, tx) = tokio::io::split(casimir_io);
                            casimir::Device::rf(id, rx, tx, rf_tx)
                        })
                        .await;
                    info!("NfcActor: scene_client.add_device returned: {:?}", add_result);
                    match add_result {
                        Ok(id) => {
                            info!(
                                "NfcActor: Sending success response to grpc-server with ID {}",
                                id
                            );
                            let _ = respond_to.send(Ok((grpc_io, id)));
                            Ok(())
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to add RF device to Casimir: {e}");
                            error!("NfcActor: {}", err_msg);
                            let _ = respond_to.send(Err(NfcError::Internal(err_msg)));
                            Ok(())
                        }
                    }
                } else {
                    let err_msg = "Casimir scene not started".to_string();
                    error!("NfcActor: {}", err_msg);
                    let _ = respond_to.send(Err(NfcError::Internal(err_msg)));
                    Ok(())
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncWriteExt, duplex};
    use tokio_stream::StreamExt;
    use tokio_util::codec::length_delimited::Builder;

    #[tokio::test]
    async fn test_nci_codec_length_adjustment() -> Result<(), Box<dyn std::error::Error>> {
        // This test documents and verifies the correct configuration of the
        // LengthDelimitedCodec used in service.rs to read NCI packets.
        // NCI header is 3 bytes (offset 2 + len 1). We must use length_adjustment(3)
        // to include the header in the returned bytes when num_skip(0) is used.

        let (mut writer, reader) = duplex(1024);
        let mut stream = Builder::new()
            .length_field_offset(2)
            .length_field_length(1)
            .length_adjustment(3) // Crucial fix!
            .num_skip(0)
            .new_read(reader);

        // Packet: header [0x40, 0x00], len 1, payload [0xaa]
        let packet = &[0x40, 0x00, 1, 0xaa];
        writer.write_all(packet).await?;
        drop(writer);

        // Verify we receive the full 4 bytes (header + payload)
        let bytes = stream.next().await.ok_or("Stream ended prematurely")??;
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes.as_ref(), packet);

        // Verify the stream terminates cleanly (EOF) on next read without looping
        assert!(stream.next().await.is_none());
        Ok(())
    }
}
