use std::time::Duration;

use actor_framework::{ActorLifecycle, ActorService, DynContext};
use async_trait::async_trait;

use crate::bluetooth_actor::BluetoothActor;

#[async_trait]
impl ActorLifecycle for BluetoothActor {
    async fn on_start(&mut self, runtime: &mut DynContext<Self>) {
        // Tick every 10ms to drive Rootcanal
        runtime.set_interval(Duration::from_millis(10));
    }

    async fn on_tick(&mut self, _runtime: &mut DynContext<Self>) {
        self.rootcanal.tick();
    }

    async fn on_stream(
        &mut self,
        id: netsim_model::ChipId,
        message: bytes::Bytes,
        _ctx: &mut DynContext<Self>,
    ) {
        if let Err(e) = self.rootcanal.receive_hci(id.0, message) {
            log::error!("Receive HCI error for chip {id}: {e}");
        }
    }

    async fn on_stream_closed(&mut self, id: netsim_model::ChipId, ctx: &mut DynContext<Self>) {
        log::info!("Stream closed for chip {id}");
        // If the stream closes, we should also ensure the sink task is aborted.
        ctx.abort(id);
        if let Err(e) = self.handle_delete(id, ctx).await {
            log::error!("Failed to delete chip {id} after stream closed: {e}");
        }
    }

    async fn on_task_closed(&mut self, id: netsim_model::ChipId, ctx: &mut DynContext<Self>) {
        log::info!("Sink task closed for chip {id}");
        // If the sink task closes, we should also ensure the stream is removed.
        ctx.remove_stream(id);
        if let Err(e) = self.handle_delete(id, ctx).await {
            log::error!("Failed to delete chip {id} after sink task closed: {e}");
        }
    }
}
