// Copyright 2025 The Android Open Source Project

use actor_framework::{ActorService, DynContext};
use futures::SinkExt;
use modem_rs::ModemSink;
use netsim_model::chip::{ChipCreate, ChipId, ChipRequest, ChipUpdate};

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
    type Entity = netsim_model::chip::Chip;
    type TypedStream = ();

    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        mut params: Self::Create,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = params.id;
        let device_id = params.device_id;

        if self.active_chips.contains_key(&chip_id) {
            return Err(CellError::ModemError(format!("Chip {} already exists", chip_id)));
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
                        log::error!("PacketSink send error: {}", e);
                    }
                }
                chip_id
            }),
        );

        // 1. Add Stream
        // Using unwrap() for stream because we checked take() above, but logic is
        // params.packet_stream.take()
        let stream = params.packet_stream.take().ok_or(CellError::MissingStreamSink)?;
        ctx.add_stream(chip_id, Box::pin(stream));

        // 2. Add to Controller directly (Sync)
        if let Err(e) = self.controller.add_modem(chip_id.0, modem_sink) {
            return Err(CellError::ModemError(format!("Controller error: {:?}", e)));
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
            log::info!("Deleting chip {}", id);
            // Remove from controller
            if let Err(e) = self.controller.remove_modem(id.0) {
                log::error!("Failed to remove modem: {:?}", e);
            }

            // Notify DeviceClient
            // We can spawn or just do it. DeviceClient methods might be async.
            let _ = self.device_client.notify_chip_removed(state.device_id, id).await;
        } else {
            // If checking fails, maybe just return Ok or Err as preferred.
            // Framework might call delete on non-existent?
        }
        Ok(())
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        if let Ok(info) = self.controller.get_modem_info(id.0) {
            Ok(Some(netsim_model::chip::Chip {
                kind: netsim_model::chip::ChipKind::CELLULAR,
                id: info.id,
                name: Some(format!("modem-{}", info.id)),
                variant: Some(netsim_model::chip::ChipVariant::Cell(netsim_model::cell::Cell {
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
        Err(CellError::ModemError("Update not implemented".into()))
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
        for (id, _state) in &self.active_chips {
            if let Ok(info) = self.controller.get_modem_info(id.0) {
                chips.push(netsim_model::chip::Chip {
                    kind: netsim_model::chip::ChipKind::CELLULAR,
                    id: info.id,
                    name: Some(format!("modem-{}", info.id)),
                    variant: Some(netsim_model::chip::ChipVariant::Cell(
                        netsim_model::cell::Cell {
                            state: if info.ringing {
                                "ringing".to_string()
                            } else {
                                "idle".to_string()
                            },
                        },
                    )),
                    ..Default::default()
                });
            }
        }
        Ok(chips)
    }
}
