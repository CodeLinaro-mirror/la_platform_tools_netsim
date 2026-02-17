use crate::error::SlirpError;
use crate::slirp_actor::SlirpActor;
use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;
use log::info;

#[async_trait]
impl ActorLifecycle<u32> for SlirpActor {
    type Error = SlirpError;

    async fn on_start(&mut self, _ctx: &mut DynContext<u32>) {
        info!("SlirpActor starting");
    }

    async fn on_tick(&mut self, _ctx: &mut DynContext<u32>) {}

    async fn on_stream(&mut self, _id: u32, msg: bytes::Bytes, _ctx: &mut DynContext<u32>) {
        if let Some(slirp) = &self.libslirp {
            slirp.input(msg);
        }
    }
    async fn on_stream_closed(&mut self, _id: u32, _ctx: &mut DynContext<u32>) {}
    async fn on_task_closed(&mut self, _id: u32, _ctx: &mut DynContext<u32>) {}
}
