use crate::bluetooth_actor::BluetoothActor;
use crate::error::BluetoothError;
use actor_framework::{ActorLifecycle, Context};
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
        if message.is_empty() {
            log::error!("Received empty HCI packet from stream for chip {id}");
            return;
        }
        if let Err(e) = self.rootcanal.receive_hci(id, message) {
            log::error!("Receive HCI error for chip {id}: {e}");
        }
    }

    async fn on_stream_closed(&mut self, id: u32) -> Result<bool, Self::Error> {
        log::info!("Stream closed for chip {id}");
        Ok(true)
    }

    async fn on_task_closed(&mut self, id: u32) -> Result<bool, Self::Error> {
        log::info!("Sink task closed for chip {id}");
        Ok(true)
    }
}
