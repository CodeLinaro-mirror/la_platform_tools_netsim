// Copyright 2026 The Android Open Source Project

use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use netsim_model::{
    chip::{Chip, ChipCreate, ChipId, ChipUpdate},
    chip_error::ChipError,
};
use tokio::sync::mpsc;

use crate::{
    uwb_actor::{run_sink_task, UwbActor},
    UwbAction, UwbActionResult,
};

#[async_trait]
impl ActorService for UwbActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = UwbAction;
    type ActionResult = UwbActionResult;
    type Error = ChipError;
    type Entity = Chip;

    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        params: Self::Create,
        ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = params.id;
        if self.chips.contains_key(&chip_id) {
            return Err(ChipError::ChipExists(chip_id.0));
        }

        let stream = params.packet_stream.expect("Packet stream is present");
        let sink = params.packet_sink.expect("Packet sink is present");

        let chip = Chip {
            id: chip_id.0,
            device_id: params.device_id,
            kind: netsim_model::chip::ChipKind::UWB,
            variant: Some(netsim_model::chip::ChipVariant::Uwb(Default::default())),
            name: Some(params.config.name),
            manufacturer: Some(params.config.manufacturer),
            product_name: Some(params.config.product_name),
            ..Default::default()
        };
        self.chips.insert(chip_id, chip);

        // Spawn a task to handle the sink.
        let (uci_tx, uci_rx) = mpsc::channel(10);
        self.uci_senders.insert(chip_id, uci_tx);
        ctx.spawn(chip_id, Box::pin(run_sink_task(sink, uci_rx, chip_id)));

        // Register the stream with the actor's context to get lifecycle events.
        ctx.add_stream(chip_id, Box::pin(stream));

        Ok(chip_id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.chips.get(&id).cloned())
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        let chip = self.chips.get_mut(&id).ok_or(ChipError::ChipNotFound(id))?;
        if let Some(pos) = update.position {
            chip.position = pos;
        }
        if let Some(orient) = update.orientation {
            chip.orientation = orient;
        }
        if let Some(netsim_model::chip::ChipVariantUpdate::Uwb(uwb_update)) = update.variant {
            if let Some(netsim_model::chip::ChipVariant::Uwb(uwb)) = &mut chip.variant {
                uwb_update.radio.apply(&mut uwb.radio);
            }
        }
        // TODO: Implement update logic to pica
        Ok(chip.clone())
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        let chip = self.chips.remove(&id).ok_or(ChipError::ChipNotFound(id))?;
        self.uci_senders.remove(&id);
        let device_client = self.device_client.clone();
        tokio::spawn(async move {
            let _ = device_client.notify_chip_removed(chip.device_id, id).await;
        });
        Ok(())
    }
    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            UwbAction::Reset { id: _ } => {
                // TODO: Implement reset
                Ok(UwbActionResult::Success)
            }
            UwbAction::GetStatistics => {
                let stats = self
                    .chips
                    .values()
                    .map(|chip| netsim_model::stats::NetsimRadioStats {
                        id: chip.id,
                        name: chip.name.clone().unwrap_or_default(),
                        tx_bytes: 0,
                        rx_bytes: 0,
                    })
                    .collect();
                Ok(UwbActionResult::Statistics(stats))
            }
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.chips.values().cloned().collect())
    }
}
