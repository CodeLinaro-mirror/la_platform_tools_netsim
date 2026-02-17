// Copyright 2025 The Android Open Source Project

use crate::error::WifiError;
use crate::wifi_actor::{WifiActor, WifiReq, WifiResponse};
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use netsim_model::chip::{Chip, ChipId};
use tokio::sync::mpsc;

#[async_trait]
impl ActorService for WifiActor {
    type Id = ChipId;
    type Create = netsim_model::chip::ChipCreate;
    type Update = netsim_model::chip::ChipUpdate;
    type Action = WifiReq;
    type ActionResult = WifiResponse;
    type Error = WifiError;
    type Entity = Chip;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        mut params: Self::Create,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        let id = id.unwrap_or(params.id);
        if self.active_chips.contains_key(&id) {
            return Err(WifiError::Internal(format!("Chip {} already exists", id)));
        }

        let stream = params
            .packet_stream
            .take()
            .ok_or(WifiError::Internal("Missing PacketStream".into()))?;
        let sink =
            params.packet_sink.take().ok_or(WifiError::Internal("Missing PacketSink".into()))?;

        // Spawn sink task
        let (tx, mut rx) = mpsc::unbounded_channel::<bytes::Bytes>();
        self.senders.insert(id, tx);

        // Define sink task: forwards bytes from channel to PacketSink
        let sink_id = id;
        _ctx.spawn(
            sink_id,
            Box::pin(async move {
                let mut sink = sink;
                while let Some(packet) = rx.recv().await {
                    if sink.send(packet).await.is_err() {
                        break;
                    }
                }
                sink_id
            }),
        );

        // Register stream with context for polling
        let mapped_stream = stream.map(move |packet| bytes::Bytes::from(packet));
        _ctx.add_stream(id, Box::pin(mapped_stream));

        let chip = Chip {
            id: id.0,
            device_id: params.device_id,
            kind: netsim_model::chip::ChipKind::WIFI,
            name: Some(params.config.name),
            manufacturer: Some(params.config.manufacturer),
            product_name: Some(params.config.product_name),
            ..Default::default()
        };
        self.active_chips.insert(id, chip);

        // Notify Medium about new chip
        self.medium.add(id.0);

        Ok(id)
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
            if let Some(netsim_model::chip::ChipVariantUpdate::Wifi(_)) = update.variant {
                // No variant specific updates
            }
            if let Some(enabled) = update.enabled {
                self.medium.set_enabled(id.0, enabled);
            }
            if let Some(pos) = update.position {
                chip.position = pos;
            }
            // Update the chip state properly
            use crate::medium::types::WifiResult;
            if let Ok(enabled) = self.medium.enabled(id.0) {
                chip.enabled = enabled;
            }
            Ok(chip.clone())
        } else {
            Err(WifiError::Internal(format!("Chip {} not found", id)))
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.handle_delete_impl(id, ctx).await
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            WifiReq::GetStatistics => {
                let mut stats = Vec::new();
                for (id, chip) in &self.active_chips {
                    let rx_count = self.medium.get_rx_count(id.0);
                    let tx_count = self.medium.get_tx_count(id.0);
                    stats.push(netsim_model::stats::NetsimRadioStats {
                        id: id.0,
                        name: chip.name.clone().unwrap_or_default(),
                        tx_bytes: tx_count as u64,
                        rx_bytes: rx_count as u64,
                        ..Default::default()
                    });
                }
                Ok(WifiResponse::Statistics(stats.into_boxed_slice()))
            }
            WifiReq::Reset { id } => {
                self.medium.reset(id.0);
                if let Some(chip) = self.active_chips.get_mut(&id) {
                    chip.enabled = true;
                }
                Ok(WifiResponse::Ok)
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
