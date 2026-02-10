// Copyright 2026 The Android Open Source Project

use actor_framework::{ActorLifecycle, ActorService, DynContext};
use async_trait::async_trait;
use bytes::Bytes;
use netsim_model::chip::ChipId;

use crate::uwb_actor::UwbActor;

#[async_trait]
#[async_trait]
impl ActorLifecycle for UwbActor {
    async fn on_start(&mut self, _ctx: &mut DynContext<Self>) {
        // No startup logic needed yet
    }

    async fn on_stream(&mut self, id: ChipId, _msg: Bytes, _ctx: &mut DynContext<Self>) {
        // Ignored for now
    }

    async fn on_stream_closed(&mut self, id: ChipId, ctx: &mut DynContext<Self>) {
        log::info!("Stream closed for chip {id}");
        // If the stream closes, we should also ensure the sink task is aborted.
        ctx.abort(id);
        if let Err(e) = self.handle_delete(id, ctx).await {
            log::warn!("Failed to delete chip {id} after stream closed: {e}");
        }
    }

    async fn on_task_closed(&mut self, id: ChipId, ctx: &mut DynContext<Self>) {
        log::info!("Sink task closed for chip {id}");
        // If the sink task closes, we should also ensure the stream is removed.
        ctx.remove_stream(id);
        if let Err(e) = self.handle_delete(id, ctx).await {
            log::warn!("Failed to delete chip {id} after sink task closed: {e}");
        }
    }
}
