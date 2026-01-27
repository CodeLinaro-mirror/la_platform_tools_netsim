// Copyright 2026 The Android Open Source Project

use crate::uwb_actor::UwbActor;
use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;
use netsim_model::chip::ChipId;

#[async_trait]
impl ActorLifecycle<ChipId> for UwbActor {
    type Error = netsim_model::chip_error::ChipError;

    async fn on_start(&mut self, _ctx: &mut DynContext<ChipId>) {
        // No startup logic needed yet
    }

    async fn on_tick(&mut self, _ctx: &mut DynContext<ChipId>) {
        // No tick logic needed yet
    }
}
