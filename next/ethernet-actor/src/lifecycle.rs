// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorLifecycle, DynContext};
use netsim_model::ChipId;
use tracing::info;

use crate::ethernet_actor::EthernetActor;

impl ActorLifecycle for EthernetActor {
    async fn on_start(&mut self, _ctx: &mut DynContext<Self>) {
        info!("EthernetActor starting");
    }

    async fn on_typed_stream(
        &mut self,
        _id: usize,
        disconnected_chip_id: ChipId,
        _ctx: &mut DynContext<Self>,
    ) {
        info!("EthernetActor notified of disconnection for chip {disconnected_chip_id}");
        if let Some(chip) = self.active_chips.remove(&disconnected_chip_id) {
            let dc = self.device_client.clone();
            let device_id = chip.device_id;
            tokio::spawn(async move {
                let _ = dc.notify_chip_removed(device_id, disconnected_chip_id).await;
            });
        }
    }
}
