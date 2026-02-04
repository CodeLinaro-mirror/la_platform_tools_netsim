// Copyright 2026 The Android Open Source Project

use std::task::Poll;

use actor_framework::{ActorLifecycle, ActorService, DynContext};
use async_trait::async_trait;
use bytes::Bytes;
use netsim_model::chip::ChipId;

use crate::uwb_actor::UwbActor;

#[async_trait]
impl ActorLifecycle for UwbActor {
    async fn on_start(&mut self, _ctx: &mut DynContext<Self>) {
        log::info!("UwbActor starting Pica run loop");
        let pica = self.pica.clone();
        // TODO(b/483090891): Use a sentinel chip ID to keep this inside the actor
        // lifecycle.
        self.pica_task = Some(tokio::spawn(async move { pica::run(&pica).await }));
    }

    async fn on_stream(&mut self, id: ChipId, msg: Bytes, _ctx: &mut DynContext<Self>) {
        if let Some(state) = self.chip_states.get(&id) {
            let _ = state.pica_sender.send(msg).await;
        } else {
            log::warn!("UCI packet received for unknown chip {id}");
        }
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

    async fn on_tick(&mut self, _ctx: &mut DynContext<Self>) {
        if let Some(task) = &mut self.pica_task {
            if let Poll::Ready(Err(err)) = futures::poll!(task) {
                panic!("Pica run loop encountered an error: {err}");
            }
        }
    }

    async fn on_shutdown(&mut self) {
        if let Some(task) = self.pica_task.take() {
            task.abort();
        }
    }
}
