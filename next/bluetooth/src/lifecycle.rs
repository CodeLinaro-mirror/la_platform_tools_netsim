use crate::bluetooth_actor::BluetoothActor;
use crate::error::BluetoothError;
use actor_framework::{ActorLifecycle, ActorService, Context};
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
impl ActorLifecycle for BluetoothActor {
    type Error = BluetoothError;

    async fn on_start(&mut self, runtime: &mut impl Context) {
        // Tick every 10ms to drive Rootcanal
        runtime.set_interval(Duration::from_millis(10));
    }

    async fn on_tick(&mut self, _runtime: &mut impl Context) {
        self.rootcanal.tick();
    }

    async fn on_stream(&mut self, id: u32, message: bytes::Bytes, _ctx: &mut impl Context) {
        if let Err(e) = self.rootcanal.receive_hci(id, message) {
            log::error!("Receive HCI error for chip {id}: {e}");
        }
    }

    async fn on_stream_closed(&mut self, id: u32, ctx: &mut impl Context) {
        log::info!("Stream closed for chip {id}");
        if let Err(e) = self.handle_delete(netsim_model::chip::ChipId(id), ctx).await {
            log::error!("Failed to delete chip {id} after stream closed: {e}");
        }
    }

    async fn on_task_closed(&mut self, id: u32, ctx: &mut impl Context) {
        log::info!("Sink task closed for chip {id}");
        if let Err(e) = self.handle_delete(netsim_model::chip::ChipId(id), ctx).await {
            log::error!("Failed to delete chip {id} after sink task closed: {e}");
        }
    }
}
