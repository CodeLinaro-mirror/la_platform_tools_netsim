// Copyright 2026 The Android Open Source Project

use crate::uwb_actor::UwbActor;
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use netsim_model::chip::{Chip, ChipCreate, ChipId, ChipUpdate};
use netsim_model::chip_error::ChipError;

#[async_trait]
impl ActorService for UwbActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = crate::UwbAction;
    type ActionResult = crate::UwbActionResult;
    type Error = ChipError;
    type Entity = Chip;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = id.unwrap_or(params.id);
        self.create_chip(params)?;
        Ok(chip_id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.active_chips.get(&id).cloned())
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        if let Some(chip) = self.active_chips.get_mut(&id) {
            if let Some(pos) = update.position {
                chip.position = pos;
            }
            if let Some(orient) = update.orientation {
                chip.orientation = orient;
            }
            if let Some(netsim_model::chip::ChipVariantUpdate::Uwb(radio_update)) = update.variant {
                if let Some(netsim_model::chip::ChipVariant::Uwb(uwb_radio)) = &mut chip.variant {
                    radio_update.apply(uwb_radio);
                }
            }
            // TODO: Implement update logic to pica
            Ok(chip.clone())
        } else {
            Err(ChipError::ChipNotFound(id))
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        let device_id = if let Some(chip) = self.active_chips.get(&id) {
            chip.device_id
        } else {
            return Err(ChipError::ChipNotFound(id));
        };
        let device_client = self.device_client.clone();
        self.cleanup_chip(id, "handle_delete", &device_client).await;
        Ok(())
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            crate::UwbAction::Reset { id: _ } => {
                // TODO: Implement reset
                Ok(crate::UwbActionResult::Success)
            }
            crate::UwbAction::GetStatistics => {
                let stats = self
                    .active_chips
                    .values()
                    .filter_map(|chip| {
                        // All chips in active_chips are UWB, but check variant just in case or use kind
                        match &chip.variant {
                            Some(netsim_model::chip::ChipVariant::Uwb(_)) => {
                                Some(netsim_model::stats::NetsimRadioStats {
                                    id: chip.id,
                                    name: chip.name.clone().unwrap_or_default(),
                                    tx_bytes: 0,
                                    rx_bytes: 0,
                                })
                            }
                            _ => None,
                        }
                    })
                    .collect();
                Ok(crate::UwbActionResult::Statistics(stats))
            }
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.active_chips.values().cloned().collect())
    }
}
