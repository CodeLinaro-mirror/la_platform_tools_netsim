// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorService, DynContext};
use futures::SinkExt;
use modem_rs::ModemSink;
use netsim_model::{
    Cell, Chip, ChipCreate, ChipError, ChipId, ChipKind, ChipUpdate, ChipVariant,
    ChipVariantUpdate, MODEM_STATE_DOWN, MODEM_STATE_IDLE, MODEM_STATE_RINGING, ModemAction,
    Quirks, Radio,
};
use tracing::{debug, error, info};

use crate::{
    CellAction, CellActionResult,
    cell_actor::{CellActor, ChipState},
    error::CellError,
};

impl ActorService for CellActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = CellAction;
    type ActionResult = CellActionResult;
    type Error = CellError;
    type Entity = Chip;
    type TypedStream = modem_rs::HostEvent;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        mut params: Self::Create,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = id.ok_or_else(|| ChipError::InvalidArguments("missing chip id".into()))?;
        let device_id = params.chip.device_id;

        if self.active_chips.contains_key(&chip_id) {
            return Err(CellError::Chip(ChipError::ChipExists(chip_id.0)));
        }

        let mut sink = params.packet_sink.take().ok_or(CellError::MissingStreamSink)?;

        // Bridge Async PacketSink to Sync ModemSink
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<bytes::Bytes>();
        let modem_sink = ModemSink::new(move |b| tx.send(b).map_err(|e| e.to_string()));

        // Spawn forwarder task
        ctx.spawn(
            chip_id,
            Box::pin(async move {
                while let Some(packet) = rx.recv().await {
                    debug!(
                        "CellActor forwarder: raw packet for chip {}: {:?}",
                        chip_id,
                        std::str::from_utf8(&packet)
                    );
                    // Note: If the underlying transport is a PTY, ensure ONLCR processing
                    // is disabled on the PTY descriptor (e.g. at PTY creation site when opened
                    // via openpty/tcsetattr for Casimir or emulator bridge) to prevent '\n' to
                    // '\r\n' expansion which corrupts the packet framing.
                    if let Err(e) = sink.send(packet).await {
                        error!("PacketSink send error: {}", e);
                    }
                }
                chip_id
            }),
        );

        // 1. Add Stream
        let stream = params.packet_stream.take().ok_or(CellError::MissingStreamSink)?;
        ctx.add_stream(chip_id, Box::pin(stream));

        // 2. Add to Controller directly (Sync)
        let (sim_type, sim_profile, quirks) = match params.chip.variant.as_ref() {
            Some(ChipVariant::Cell(cell)) => (cell.sim_type, cell.sim_profile.clone(), cell.quirks),
            _ => (None, None, Quirks::default()),
        };
        if let Err(e) =
            self.controller.add_modem(chip_id.0, modem_sink, sim_type, sim_profile, quirks)
        {
            return Err(CellError::ModemError(e));
        }

        self.active_chips.insert(chip_id, ChipState { device_id, enabled: true });

        Ok(chip_id)
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        // Remove from local state
        if let Some(state) = self.active_chips.remove(&id) {
            info!("Deleting chip {}", id);
            // Remove from controller
            if let Err(e) = self.controller.remove_modem(id.0) {
                error!("Failed to remove modem: {:?}", e);
            }

            // Notify DeviceClient asynchronously
            let dc = self.device_client.clone();
            let device_id = state.device_id;
            tokio::spawn(async move {
                let _ = dc.notify_chip_removed(device_id, id).await;
            });
        }
        Ok(())
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        if let Ok(info) = self.controller.get_modem_info(id.0) {
            let enabled = self.active_chips.get(&id).map(|s| s.enabled).unwrap_or(true);
            Ok(Some(Chip {
                kind: ChipKind::CELLULAR,
                id: info.id,
                name: format!("modem-{}", info.id),
                enabled,
                variant: Some(ChipVariant::Cell(Cell {
                    radio: Radio { state: Some(enabled), ..Default::default() },
                    state: if !enabled {
                        MODEM_STATE_DOWN.to_string()
                    } else if info.ringing {
                        MODEM_STATE_RINGING.to_string()
                    } else {
                        MODEM_STATE_IDLE.to_string()
                    },
                    sim_type: None,
                    sim_profile: None,
                    quirks: info.quirks,
                    sms_count: info.sms_count,
                    rssi: info.rssi,
                    ber: info.ber,
                    voice_registration: info.voice_registration,
                    data_registration: info.data_registration,
                    active_calls: info.calls,
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
            .ok_or_else(|| CellError::Chip(ChipError::ChipNotFound(id)))?;

        if let Some(enabled) = update.enabled {
            state.enabled = enabled;
        }
        if let Some(ChipVariantUpdate::Cell(cell_update)) = &update.variant {
            if let Some(s) = &cell_update.state {
                state.enabled = s != "down";
            }
            if let Some(enabled) = cell_update.radio.state {
                state.enabled = enabled;
            }
        }

        self.handle_get(id, ctx)
            .await?
            .ok_or_else(|| CellError::Chip(netsim_model::ChipError::ChipNotFound(id)))
    }

    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        let chip_id = id.ok_or_else(|| {
            CellError::Chip(ChipError::InvalidArguments("missing chip id".into()))
        })?;

        if !self.active_chips.contains_key(&chip_id) {
            return Err(CellError::Chip(ChipError::ChipNotFound(chip_id)));
        }

        let modem_action = match action {
            CellAction::IncomingCall { number } => {
                if !is_valid_phone_number(&number) {
                    return Err(CellError::Chip(ChipError::InvalidArguments(
                        format!("Invalid phone number: {number}").into(),
                    )));
                }
                ModemAction::IncomingCall { target_id: chip_id, number }
            }
            CellAction::UpdateCall => ModemAction::UpdatePhysicalChannelConfigs { id: chip_id },
            CellAction::EndCall => ModemAction::RemoteHangup { id: chip_id },
            CellAction::ReceiveSms { sender, text } => {
                if !is_valid_sms_sender(&sender) {
                    return Err(CellError::Chip(ChipError::InvalidArguments(
                        format!("Invalid SMS sender: {sender}").into(),
                    )));
                }
                ModemAction::IncomingSms { id: chip_id, sender, text }
            }
            CellAction::ReceivePdu { pdu } => {
                if !is_valid_hex_pdu(&pdu) {
                    return Err(CellError::Chip(ChipError::InvalidArguments(
                        format!("Invalid PDU format (must be even-length hex string): {pdu}")
                            .into(),
                    )));
                }
                ModemAction::IncomingPdu { id: chip_id, pdu }
            }
            CellAction::SetSignalStrength { rssi, ber } => {
                let rssi_u8 = u8::try_from(rssi).map_err(|e| {
                    CellError::Chip(ChipError::InvalidArguments(
                        format!("rssi out of range ({}): {}", rssi, e).into(),
                    ))
                })?;
                let ber_u8 = u8::try_from(ber).map_err(|e| {
                    CellError::Chip(ChipError::InvalidArguments(
                        format!("ber out of range ({}): {}", ber, e).into(),
                    ))
                })?;
                ModemAction::SetSignalStrength { id: chip_id, rssi: rssi_u8, ber: ber_u8 }
            }
            CellAction::SetVoiceRegistration { status } => {
                ModemAction::SetVoiceRegistration { id: chip_id, status }
            }
            CellAction::SetDataRegistration { status } => {
                ModemAction::SetDataRegistration { id: chip_id, status }
            }
            CellAction::RemoteAnswer => ModemAction::RemoteAnswer { id: chip_id },
            CellAction::RemoteHold { on_hold } => ModemAction::RemoteHold { id: chip_id, on_hold },
            CellAction::SetSimStatus { present } => {
                ModemAction::SetSimStatus { id: chip_id, present }
            }
            CellAction::SetNetworkTechnology { tech } => {
                ModemAction::SetNetworkTechnology { id: chip_id, tech }
            }
            CellAction::SetOperator { operator } => {
                ModemAction::SetOperator { id: chip_id, operator }
            }
        };

        self.controller.perform_action(modem_action)?;

        Ok(CellActionResult::Success)
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        let mut chips = Vec::new();
        for (id, state) in &self.active_chips {
            if let Ok(info) = self.controller.get_modem_info(id.0) {
                chips.push(Chip {
                    kind: ChipKind::CELLULAR,
                    id: info.id,
                    name: format!("modem-{}", info.id),
                    enabled: state.enabled,
                    variant: Some(ChipVariant::Cell(Cell {
                        radio: Radio { state: Some(state.enabled), ..Default::default() },
                        state: if !state.enabled {
                            MODEM_STATE_DOWN.to_string()
                        } else if info.ringing {
                            MODEM_STATE_RINGING.to_string()
                        } else {
                            MODEM_STATE_IDLE.to_string()
                        },
                        sim_type: None,
                        sim_profile: None,
                        quirks: info.quirks,
                        sms_count: info.sms_count,
                        rssi: info.rssi,
                        ber: info.ber,
                        voice_registration: info.voice_registration,
                        data_registration: info.data_registration,
                        active_calls: info.calls,
                    })),
                    ..Default::default()
                });
            }
        }
        Ok(chips)
    }
}

fn is_valid_phone_number(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_digit()
                || c == '+'
                || c == '-'
                || c == ' '
                || c == '('
                || c == ')'
                || c == '*'
                || c == '#'
        })
}

fn is_valid_sms_sender(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == ' ' || c == '(' || c == ')'
        })
}

fn is_valid_hex_pdu(s: &str) -> bool {
    !s.is_empty() && s.len().is_multiple_of(2) && s.chars().all(|c| c.is_ascii_hexdigit())
}
