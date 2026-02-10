// Copyright 2026 The Android Open Source Project

use actor_framework::{ActorLifecycle, ActorService, DynContext};
use async_trait::async_trait;
use pica::PicaEvent;

use crate::uwb_actor::UwbActor;

#[async_trait]
impl ActorLifecycle for UwbActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        log::info!("UwbActor starting Pica run loop");
        let pica = self.pica.clone();
        // TODO(b/483090891): Use a sentinel chip ID to keep this inside the actor
        // lifecycle.
        self.pica_task = Some(tokio::spawn(async move { pica::run(&pica).await }));

        ctx.set_interval(Self::TICK_INTERVAL);
    }

    async fn on_tick(&mut self, ctx: &mut DynContext<Self>) {
        while let Ok(event) = self.pica_events.try_recv() {
            // Received in response to either `PicaCommand::Disconnect` or stream/sink
            // closure.
            let PicaEvent::Disconnected { handle, .. } = event else { continue };
            let Some(id) = self.handle_to_chip.remove(&handle) else { continue };

            let _ = self.handle_delete(id, ctx).await;
        }
    }

    async fn on_shutdown(&mut self) {
        if let Some(task) = self.pica_task.take() {
            task.abort();
        }
    }
}
