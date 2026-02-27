// Copyright 2026 The Android Open Source Project

use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{FutureExt, SinkExt, StreamExt};
use netsim_model::{
    chip::{Chip, ChipCreate, ChipUpdate, ChipVariant},
    chip_error::ChipError,
    ChipId, ChipKind,
};
use pdl_runtime::Packet;
use pica::{packets::uci, PicaCommand, PicaEvent};

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
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = params.id;
        if self.chip_to_handle.contains_key(&chip_id) {
            return Err(ChipError::ChipExists(chip_id.0));
        }

        let chip = Chip {
            id: chip_id.0,
            device_id: params.device_id,
            kind: ChipKind::UWB,
            variant: Some(ChipVariant::Uwb(Default::default())),
            name: Some(params.config.name),
            manufacturer: Some(params.config.manufacturer),
            product_name: Some(params.config.product_name),
            ..Default::default()
        };

        let stream =
            params.packet_stream.expect("Packet stream is present").map(|b| b.to_vec()).boxed();

        // Pica wants a Sink<Vec<u8>>.
        let sink = Box::pin(
            params
                .packet_sink
                .expect("Packet sink is present")
                .with(|v| async move { Ok(Bytes::from(v)) }),
        );

        // Clear unrelated events to prevent a lagged error
        self.pica_connect_events.resubscribe();

        let _ = self.pica_commands.send(PicaCommand::Connect(stream, sink)).await;
        // Wait for add to complete. This guarantees the chip exists by the time any
        // actions are performed on it.
        let handle = loop {
            if let PicaEvent::Connected { handle, .. } = self
                .pica_connect_events
                .recv()
                .await
                .map_err(|_| ChipError::Internal("pica shutdown unexpectedly".to_string()))?
            {
                break handle;
            }
        };

        self.chip_states.write().unwrap().insert(handle, UwbChipState { chip });
        self.chip_to_handle.insert(chip_id, handle);

        Ok(chip_id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        let handle = self.chip_to_handle.get(&id).ok_or(ChipError::ChipNotFound(id))?;
        Ok(self.chip_states.read().unwrap().get(handle).map(|state| state.chip.clone()))
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        let handle = self.chip_to_handle.get(&id).ok_or(ChipError::ChipNotFound(id))?;
        let mut chips = self.chip_states.write().unwrap();
        let state = chips.get_mut(handle).ok_or(ChipError::ChipNotFound(id))?;

        state.apply(update);

        Ok(state.chip.clone())
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        let handle = self.chip_to_handle.remove(&id).ok_or(ChipError::ChipNotFound(id))?;
        let state = self.chip_states.write().unwrap().remove(&handle).unwrap();

        // Disconnect from Pica. This completes asynchronously as we'll receive a
        // `PicaEvent::Disconnected` event in the lifecycle `on_tick`.
        let _ = self.pica_commands.send(PicaCommand::Disconnect(handle)).await;

        let device_client = self.device_client.clone();
        let device_id = state.chip.device_id;
        ctx.spawn(
            id,
            async move {
                let _ = device_client.notify_chip_removed(device_id, id).await;
                id
            }
            .boxed(),
        );

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
                if let Some(handle) = self.chip_to_handle.get(&id) {
                    let reset_cmd =
                        uci::CoreDeviceResetCmd { reset_config: uci::ResetConfig::UwbsReset };
                    let _ = self
                        .pica_commands
                        .send(PicaCommand::UciPacket(
                            *handle,
                            reset_cmd.encode_to_vec().expect("encoding succeeds"),
                        ))
                        .await;
                }
                Ok(UwbActionResult::Success)
            }
            UwbAction::GetStatistics => {
                let stats = self
                    .chip_states
                    .read()
                    .unwrap()
                    .values()
                    .map(|state| netsim_model::stats::NetsimRadioStats {
                        id: state.chip.id,
                        name: state.chip.name.clone().unwrap_or_default(),
                        kind: ChipKind::UWB,
                        tx_count: 0,
                        rx_count: 0,
                        tx_bytes: 0,
                        rx_bytes: 0,
                        ..Default::default()
                    })
                    .collect();
                Ok(UwbActionResult::Statistics(stats))
            }
            UwbAction::StartRanging { id, session_id } => {
                if let Some(handle) = self.chip_to_handle.get(&id) {
                    let _ =
                        self.pica_commands.send(PicaCommand::Ranging(*handle, session_id)).await;
                }
                Ok(UwbActionResult::Success)
            }
            UwbAction::StopRanging { id, session_id } => {
                if let Some(handle) = self.chip_to_handle.get(&id) {
                    let stop_cmd = uci::SessionStopCmd { session_id };
                    let _ = self
                        .pica_commands
                        .send(PicaCommand::UciPacket(
                            *handle,
                            stop_cmd.encode_to_vec().expect("encoding succeeds"),
                        ))
                        .await;
                }
                Ok(UwbActionResult::Success)
            }
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.chip_states.read().unwrap().values().map(|state| state.chip.clone()).collect())
    }
}
