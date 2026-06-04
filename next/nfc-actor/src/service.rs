// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorService, DynContext};
use netsim_model::{ChipCreate, ChipError, ChipId, ChipRequest, ChipUpdate};
use tracing::info;

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

        // Register the packet stream to receive packets from the guest
        if let Some(stream) = params.packet_stream.take() {
            ctx.add_stream(chip_id, Box::pin(stream));
        } else {
            return Err(NfcError::MissingStreamSink);
        }

        // We ignore packet_sink for now as we don't have a simulator to send packets
        // back. In a real implementation, we would bridge this to Casimir.

        self.active_chips.insert(chip_id, ChipState { device_id, enabled: true });
        info!("NFC chip {} created for device {}", chip_id, device_id);

        Ok(chip_id)
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        if let Some(state) = self.active_chips.remove(&id) {
            info!("Deleting NFC chip {}", id);
            // Notify DeviceClient
            let _ = self.device_client.notify_chip_removed(state.device_id, id).await;
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
