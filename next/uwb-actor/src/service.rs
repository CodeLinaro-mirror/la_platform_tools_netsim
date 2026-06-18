// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use actor_framework::{ActorService, DynContext};
use bytes::Bytes;
use futures::{FutureExt, SinkExt, StreamExt};
use netsim_model::{Chip, ChipCreate, ChipError, ChipId, ChipUpdate};
use pdl_runtime::Packet;
use pica::{PicaCommand, PicaEvent, packets::uci};

use crate::{
    UwbAction, UwbActionResult,
    error::UwbError,
    uwb_actor::{UwbActor, UwbChipState},
};

impl ActorService for UwbActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = UwbAction;
    type ActionResult = UwbActionResult;
    type Error = UwbError;
    type Entity = Chip;
    type TypedStream = ();

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = id.ok_or(ChipError::InvalidArguments(Box::from("missing chip id")))?;
        if self.chip_to_handle.contains_key(&chip_id) {
            return Err(ChipError::ChipExists(chip_id.0).into());
        }

        let mut chip = params.chip;
        chip.id = chip_id.0;

        let stream =
            params.packet_stream.ok_or(UwbError::PacketStreamMissing)?.map(|b| b.to_vec()).boxed();

        let p2p_tx_count = Arc::new(AtomicU64::new(0));
        let p2p_rx_count = Arc::new(AtomicU64::new(0));
        let tx_clone = p2p_tx_count.clone();
        let rx_clone = p2p_rx_count.clone();

        // Pica wants a Sink<Vec<u8>>.
        let sink = Box::pin(params.packet_sink.ok_or(UwbError::PacketSinkMissing)?.with(
            move |v: Vec<u8>| {
                if let Some(count) = count_p2p_ranging_measurements(&v).ok().filter(|&c| c > 0) {
                    // A single ranging cycle involves multiple packets transmitted and
                    // received. Incrementing both TX and RX by the same
                    // cycle count is a reasonable proxy for activity,
                    // but it's an approximation of P2P activity.
                    tx_clone.fetch_add(count, Ordering::Relaxed);
                    rx_clone.fetch_add(count, Ordering::Relaxed);
                }
                async move { Ok(Bytes::from(v)) }
            },
        ));

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
                .map_err(|err| UwbError::PicaShutdown(Box::new(err)))?
            {
                break handle;
            }
        };

        self.chip_states
            .write()
            .unwrap()
            .insert(handle, UwbChipState { chip: chip.clone(), p2p_tx_count, p2p_rx_count });
        self.initial_chips.insert(chip_id, chip);
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

        update.apply(&mut state.chip);

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
                let handle = self.chip_to_handle.get(&id).ok_or(ChipError::ChipNotFound(id))?;
                let initial_chip =
                    self.initial_chips.get(&id).ok_or(ChipError::ChipNotFound(id))?;
                let chip = {
                    let mut chips = self.chip_states.write().unwrap();
                    let state = chips.get_mut(handle).ok_or(ChipError::ChipNotFound(id))?;
                    state.chip = initial_chip.clone();
                    state.chip.clone()
                };
                let reset_cmd =
                    uci::CoreDeviceResetCmd { reset_config: uci::ResetConfig::UwbsReset };
                let _ = self
                    .pica_commands
                    .send(PicaCommand::UciPacket(
                        *handle,
                        reset_cmd.encode_to_vec().expect("encoding succeeds"),
                    ))
                    .await;

                Ok(UwbActionResult::Chip(chip))
            }
            UwbAction::GetStatistics => {
                let stats = self
                    .chip_states
                    .read()
                    .unwrap()
                    .values()
                    .map(|state| {
                        let mut radio_stats = netsim_model::NetsimRadioStats::default();
                        radio_stats.id = state.chip.id;
                        radio_stats.name = state.chip.name.clone();
                        radio_stats.kind = netsim_model::RadioKind::Uwb;
                        radio_stats.p2p_tx_count =
                            state.p2p_tx_count.load(std::sync::atomic::Ordering::Relaxed);
                        radio_stats.p2p_rx_count =
                            state.p2p_rx_count.load(std::sync::atomic::Ordering::Relaxed);
                        radio_stats
                    })
                    .collect();
                Ok(UwbActionResult::Statistics(stats))
            }
            #[cfg(any(test, feature = "testing"))]
            UwbAction::StartRanging { id, session_id } => {
                if let Some(handle) = self.chip_to_handle.get(&id) {
                    let _ =
                        self.pica_commands.send(PicaCommand::Ranging(*handle, session_id)).await;
                }
                Ok(UwbActionResult::Success)
            }
            #[cfg(any(test, feature = "testing"))]
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

fn count_p2p_ranging_measurements(bytes: &[u8]) -> Result<u64, ()> {
    if bytes.len() < 4 {
        return Err(());
    }
    use pica::packets::uci::{
        ControlPacket, ControlPacketChild, ExtendedMacOwrAoaSessionInfoNtf,
        ExtendedMacTwoWaySessionInfoNtf, SessionControlPacketChild, ShortMacOwrAoaSessionInfoNtf,
        ShortMacTwoWaySessionInfoNtf,
    };

    let Ok((packet, _)) = ControlPacket::decode(bytes) else {
        return Err(());
    };
    let Ok(ControlPacketChild::SessionControlPacket(ntf)) = packet.specialize() else {
        return Err(());
    };
    let Ok(SessionControlPacketChild::SessionInfoNtf(info)) = ntf.specialize() else {
        return Err(());
    };

    if let Ok(m) = ShortMacTwoWaySessionInfoNtf::try_from(&info) {
        return Ok(m.two_way_ranging_measurements.len() as u64);
    }
    if let Ok(m) = ExtendedMacTwoWaySessionInfoNtf::try_from(&info) {
        return Ok(m.two_way_ranging_measurements.len() as u64);
    }
    if let Ok(m) = ShortMacOwrAoaSessionInfoNtf::try_from(&info) {
        return Ok(m.owr_aoa_ranging_measurements.len() as u64);
    }
    if let Ok(m) = ExtendedMacOwrAoaSessionInfoNtf::try_from(&info) {
        return Ok(m.owr_aoa_ranging_measurements.len() as u64);
    }
    Err(())
}

#[cfg(test)]
mod tests {
    use pica::packets::uci::{
        ControlPacket, SessionControlPacket, SessionInfoNtf, SessionInitCmd, SessionType,
        ShortAddressTwoWayRangingMeasurement, ShortMacTwoWaySessionInfoNtf, Status,
    };

    use super::*;

    #[test]
    fn test_parsing_empty() {
        assert_eq!(count_p2p_ranging_measurements(&[]), Err(()));
        assert_eq!(count_p2p_ranging_measurements(&[1, 2]), Err(()));
        assert_eq!(count_p2p_ranging_measurements(&[1, 2, 3]), Err(()));
        assert_eq!(count_p2p_ranging_measurements(&[0; 10]), Err(()));
    }

    #[test]
    fn test_parsing_valid_packet() {
        let ntf = ShortMacTwoWaySessionInfoNtf {
            sequence_number: 0,
            session_token: 0,
            rcr_indicator: 0,
            current_ranging_interval: 0,
            two_way_ranging_measurements: vec![
                ShortAddressTwoWayRangingMeasurement {
                    mac_address: 0,
                    status: Status::Ok,
                    nlos: 0,
                    distance: 0,
                    aoa_azimuth: 0,
                    aoa_azimuth_fom: 0,
                    aoa_elevation: 0,
                    aoa_elevation_fom: 0,
                    aoa_destination_azimuth: 0,
                    aoa_destination_azimuth_fom: 0,
                    aoa_destination_elevation: 0,
                    aoa_destination_elevation_fom: 0,
                    slot_index: 0,
                    rssi: 0,
                },
                ShortAddressTwoWayRangingMeasurement {
                    mac_address: 1,
                    status: Status::Ok,
                    nlos: 0,
                    distance: 10,
                    aoa_azimuth: 0,
                    aoa_azimuth_fom: 0,
                    aoa_elevation: 0,
                    aoa_elevation_fom: 0,
                    aoa_destination_azimuth: 0,
                    aoa_destination_azimuth_fom: 0,
                    aoa_destination_elevation: 0,
                    aoa_destination_elevation_fom: 0,
                    slot_index: 1,
                    rssi: 0,
                },
            ],
            vendor_data: vec![],
        };

        let parent: SessionInfoNtf = ntf.try_into().unwrap();
        let root: SessionControlPacket = parent.try_into().unwrap();
        let final_pkt: ControlPacket = root.try_into().unwrap();

        let mut bytes = Vec::new();
        final_pkt.encode(&mut bytes).unwrap();

        assert_eq!(count_p2p_ranging_measurements(&bytes), Ok(2));
    }

    #[test]
    fn test_parsing_invalid_packet() {
        // Construct a SessionInitCmd (valid ControlPacket, but not a SessionInfoNtf)
        let cmd = SessionInitCmd { session_id: 1, session_type: SessionType::FiraRangingSession };
        let final_pkt: ControlPacket = cmd.try_into().unwrap();

        let mut bytes = Vec::new();
        final_pkt.encode(&mut bytes).unwrap();

        assert_eq!(count_p2p_ranging_measurements(&bytes), Err(()));
    }
}
