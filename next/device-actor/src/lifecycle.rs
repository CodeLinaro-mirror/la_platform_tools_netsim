use crate::device_actor::DeviceActor;
use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;
use std::time::Duration;

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
        // If we have devices, we are active.
        if !self.devices.is_empty() {
            return;
        }

        // Case 1: Waiting for first device (Startup Timeout)
        if !self.has_seen_device {
            if let Some(timeout) = self.startup_timeout {
                if self.start_time.elapsed() > timeout {
                    log::info!("DeviceActor: Startup timeout reached (no devices connected), shutting down");
                    ctx.shutdown();
                }
            }
            return;
        }

        // Case 2: Devices were present but now empty (Idle Timeout)
        if let Some(empty_time) = self.last_empty_time {
            if let Some(timeout) = self.idle_timeout {
                if empty_time.elapsed() > timeout {
                    log::info!(
                        "DeviceActor: Idle timeout reached (last device removed), shutting down"
                    );
                    ctx.shutdown();
                }
            }
        }
    }
}
