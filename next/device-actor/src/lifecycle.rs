use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;

use crate::device_actor::DeviceActor;

#[async_trait]
impl ActorLifecycle for DeviceActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        let Some(timeout) = self.startup_timeout else {
            return;
        };

        log::info!("DeviceActor: Scheduling startup timeout for {:?}", timeout);
        let key = ctx.run_later(timeout, Box::new(Self::on_startup_timeout));
        self.startup_timer = Some(key);
    }
}
