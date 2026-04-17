use actor_framework::{ActorLifecycle, DynContext};

use crate::device_actor::DeviceActor;

impl ActorLifecycle for DeviceActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        if let Some(client) = self.self_client.clone() {
            let interval = self.stats_interval;
            ctx.spawn(
                device_api::DeviceId(0),
                Box::pin(async move {
                    loop {
                        tokio::time::sleep(interval).await;
                        if let Err(e) = client.save_stats().await {
                            log::warn!("Failed to auto-save stats: {}", e);
                        }
                    }
                    #[allow(unreachable_code)]
                    device_api::DeviceId(0)
                }),
            );
        } else {
            log::warn!("DeviceActor started without self_client! Stats will not be auto-saved.");
        }

        let Some(timeout) = self.startup_timeout else {
            return;
        };

        log::info!("DeviceActor: Scheduling startup timeout for {:?}", timeout);
        let key = ctx.run_later(timeout, Box::new(Self::on_startup_timeout));
        self.startup_timer = Some(key);
    }

    async fn on_shutdown(&mut self) {
        // Ensure any pending detached background write finishes safely.
        if let Some(task) = self.stats_write_task.take() {
            log::info!("DeviceActor: Awaiting pending background stats write prior to shutdown");
            let _ = task.await;
        }

        // Create a final save task
        self.save_stats_async().await;
        if let Some(task) = self.stats_write_task.take() {
            log::info!("DeviceActor: Awaiting final stats write");
            let _ = task.await;
        }
    }
}
