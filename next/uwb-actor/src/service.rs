// Copyright 2026 The Android Open Source Project

use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use netsim_model::{
    chip::{Chip, ChipCreate, ChipId, ChipUpdate, ChipVariant, ChipVariantUpdate},
    chip_error::ChipError,
};
use pica::PicaCommand;
use tokio::sync::mpsc;

use crate::{
    uwb_actor::{UwbActor, UwbChipState},
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
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = params.id;
        if self.chip_states.contains_key(&chip_id) {
            return Err(ChipError::ChipExists(chip_id.0));
        }

        // TODO(b/483090273): pass this to Pica directly
        let stream = params.packet_stream.expect("Packet stream is present");
        let sink = params.packet_sink.expect("Packet sink is present");

        let chip = Chip {
            id: chip_id.0,
            device_id: params.device_id,
            kind: netsim_model::chip::ChipKind::UWB,
            variant: Some(ChipVariant::Uwb(Default::default())),
            name: Some(params.config.name),
            manufacturer: Some(params.config.manufacturer),
            product_name: Some(params.config.product_name),
            ..Default::default()
        };

        // Add chip to Pica
        let (pica_sender, pica_rx) = mpsc::channel::<bytes::Bytes>(10);
        let pica_stream =
            tokio_stream::wrappers::ReceiverStream::new(pica_rx).map(|b| b.to_vec()).boxed();

        // Pica wants a Sink<Vec<u8>>.
        let pica_sink = Box::pin(sink.with(|v| async { Ok(Bytes::from(v)) }));

        let pica_handle = self
            .pica
            .lock()
            .unwrap()
            .add_device(pica_stream, pica_sink)
            .map_err(|e| ChipError::Internal(e.to_string()))?;

        self.chip_states.insert(chip_id, UwbChipState { chip, pica_sender, pica_handle });

        // The Pica stream is wrapped and registered to get events via the actor
        // lifecycle, but we should directly use Pica's event broadcast (b/483090273).
        ctx.add_stream(chip_id, stream.boxed());

        Ok(chip_id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.chip_states.get(&id).map(|state| state.chip.clone()))
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        let state = self.chip_states.get_mut(&id).ok_or(ChipError::ChipNotFound(id))?;
        if let Some(pos) = update.position {
            state.chip.position = pos;
        }
        if let Some(orient) = update.orientation {
            state.chip.orientation = orient;
        }
        match (update.variant, &mut state.chip.variant) {
            (Some(ChipVariantUpdate::Uwb(uwb_update)), Some(ChipVariant::Uwb(uwb_radio))) => {
                uwb_update.radio.apply(&mut uwb_radio.radio);
            }
            (Some(other), _) => {
                log::warn!("Received unexpected update for chip {id}: {other:?}");
            }
            (None, _) => {}
        }
        Ok(state.chip.clone())
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        let state = self.chip_states.remove(&id).ok_or(ChipError::ChipNotFound(id))?;
        let cmd_tx = { self.pica.lock().unwrap().commands() };
        let _ = cmd_tx.send(PicaCommand::Disconnect(state.pica_handle)).await;
        let device_client = self.device_client.clone();
        let device_id = state.chip.device_id;
        tokio::spawn(async move {
            let _ = device_client.notify_chip_removed(device_id, id).await;
        });
        Ok(())
    }
    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            UwbAction::Reset { id } => {
                if let Some(state) = self.chip_states.get(&id) {
                    let _handle = state.pica_handle;
                    let _cmd_tx = self.pica.lock().unwrap().commands();
                    // TODO: implement reset
                }
                Ok(UwbActionResult::Success)
            }
            UwbAction::GetStatistics => {
                let stats = self
                    .chip_states
                    .values()
                    .map(|state| netsim_model::stats::NetsimRadioStats {
                        id: state.chip.id,
                        name: state.chip.name.clone().unwrap_or_default(),
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
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.chip_states.values().map(|state| state.chip.clone()).collect())
    }
}
