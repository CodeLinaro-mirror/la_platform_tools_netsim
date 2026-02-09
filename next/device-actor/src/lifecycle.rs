use std::time::Duration;

use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;

use crate::device_actor::DeviceActor;

#[async_trait]
impl ActorLifecycle<device_api::DeviceId> for DeviceActor {
    type Error = crate::error::DeviceError;

    async fn on_start(&mut self, ctx: &mut DynContext<device_api::DeviceId>) {
        if self.startup_timeout.is_some() || self.idle_timeout.is_some() {
            // Tick every 100ms or so to check timeouts
            ctx.set_interval(Duration::from_millis(100));
        }
    }

    async fn on_tick(&mut self, ctx: &mut DynContext<device_api::DeviceId>) {
        // Shutdown immediately if we've had devices previously but now have zero
        if self.has_seen_device {
            // Only shutdown if an idle timeout is configured (ignores if --no-shutdown is
            // passed)
            if self.last_empty_time.is_some() && self.idle_timeout.is_some() {
                log::info!("DeviceActor: Last device disconnected. Stopping immediately.");
                ctx.shutdown();
                return;
            }
        } else {
            // Initial grace period: Shutdown if no devices seen within startup timeout (or
            // idle_timeout fallback)
            let period = self.startup_timeout.or(self.idle_timeout);
            if let Some(timeout) = period {
                if self.start_time.elapsed() > timeout {
                    log::info!(
                        "DeviceActor: Startup timeout reached without any devices. Stopping."
                    );
                    ctx.shutdown();
                    return;
                }
            }
        }
    }
}
