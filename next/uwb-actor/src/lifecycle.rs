// Copyright 2026 The Android Open Source Project

use actor_framework::{ActorLifecycle, ActorService, DynContext};
use futures::FutureExt;
use log::warn;
use netsim_model::ChipId;
use pica::PicaEvent;
use tokio::sync::broadcast::error::TryRecvError;

use crate::uwb_actor::UwbActor;

const PICA_SENTINEL_CHIP_ID: ChipId = ChipId(u32::MAX);

impl ActorLifecycle for UwbActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        log::info!("UwbActor starting Pica run loop");

        let pica = self.pica.take().expect("lifecycle starts only once");
        ctx.spawn(
            PICA_SENTINEL_CHIP_ID,
            async move {
                if let Err(err) = pica.run().await {
                    panic!("Pica run loop failed: {err}");
                }
                PICA_SENTINEL_CHIP_ID
            }
            .boxed(),
        );
        ctx.set_interval(Self::TICK_INTERVAL);
    }

    async fn on_tick(&mut self, ctx: &mut DynContext<Self>) {
        loop {
            match self.pica_on_tick_events.try_recv() {
                Ok(PicaEvent::Disconnected { handle, .. }) => {
                    // Received in response to either `PicaCommand::Disconnect` or stream/sink
                    // closure.
                    let id = self
                        .chip_states
                        .read()
                        .unwrap()
                        .get(&handle)
                        .map(|state| ChipId(state.chip.id));
                    if let Some(id) = id {
                        let _ = self.handle_delete(id, ctx).await;
                    }
                }
                Ok(PicaEvent::Connected { .. }) => {}
                Err(TryRecvError::Lagged(skipped)) => {
                    warn!("UWB actor `on_tick` is too slow -- {skipped} messages were missed from Pica. There may be stale chips.");
                    continue;
                }
                Err(TryRecvError::Empty | TryRecvError::Closed) => {
                    break;
                }
            }
        }
    }

    async fn on_shutdown(&mut self) {
        // Pica is running within the actor context and does not need to be
        // explicitly shut down.
    }
}
