// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorService, DynContext};
use futures::SinkExt;
use modem_rs::ModemSink;
use netsim_model::{ChipCreate, ChipError, ChipId, ChipRequest, ChipUpdate};
use tracing::{error, info};

use crate::{
    cell_actor::{CellActor, ChipState},
    error::CellError,
};

impl ActorService for CellActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = ChipRequest;
    type ActionResult = ();
    type Error = CellError;
    type Entity = netsim_model::Chip;
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
        if let Err(e) = self.controller.add_modem(chip_id.0, modem_sink) {
            return Err(CellError::ModemError(e));
        }

        self.active_chips.insert(chip_id, ChipState { device_id });

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
        if let Ok(info) = self.controller.get_modem_info(id.0) {
            Ok(Some(netsim_model::Chip {
                kind: netsim_model::ChipKind::CELLULAR,
                id: info.id,
                name: format!("modem-{}", info.id),
                variant: Some(netsim_model::ChipVariant::Cell(netsim_model::Cell {
                    state: if info.ringing { "ringing".to_string() } else { "idle".to_string() },
                })),
                ..Default::default()
            }))
        } else {
            Ok(None)
        }
    }

    async fn handle_update(
        &mut self,
        _id: Self::Id,
        _update: Self::Update,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        Err(CellError::Chip(ChipError::Unsupported))
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
        for id in self.active_chips.keys() {
            if let Ok(info) = self.controller.get_modem_info(id.0) {
                chips.push(netsim_model::Chip {
                    kind: netsim_model::ChipKind::CELLULAR,
                    id: info.id,
                    name: format!("modem-{}", info.id),
                    variant: Some(netsim_model::ChipVariant::Cell(netsim_model::Cell {
                        state: if info.ringing {
                            "ringing".to_string()
                        } else {
                            "idle".to_string()
                        },
                    })),
                    ..Default::default()
                });
            }
        }
        Ok(chips)
    }
}
