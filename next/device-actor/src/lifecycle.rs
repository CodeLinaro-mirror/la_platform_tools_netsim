use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;

use crate::device_actor::DeviceActor;

#[async_trait]
impl ActorLifecycle for DeviceActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        self.schedule_periodic_stats(ctx);
        let Some(timeout) = self.startup_timeout else {
            return;
        };

        log::info!("DeviceActor: Scheduling startup timeout for {:?}", timeout);
        let key = ctx.run_later(timeout, Box::new(Self::on_startup_timeout));
        self.startup_timer = Some(key);
    }

    async fn on_shutdown(&mut self) {
        if let Some(key) = self.stats_timer.take() {
            log::info!("DeviceActor: Cancelling periodic stats timer on shutdown");
        }

        // Ensure any pending detached background write finishes safely.
        if let Some(task) = self.stats_write_task.take() {
            log::info!("DeviceActor: Awaiting pending background stats write prior to shutdown");
            let _ = task.await;
        }

        let base_stats = self.stats.get_base_stats();
        let path = self.stats.stats_path.clone();
        if let Err(e) = crate::stats::write_combined_stats(base_stats, path) {
            log::error!("DeviceActor on_shutdown: {}", e);
        }
    }
}
