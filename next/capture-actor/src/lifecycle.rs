use std::time::Duration;

use actor_framework::{ActorLifecycle, DynContext};

use crate::capture_actor::CaptureActor;

/// Periodically flush writers, ensuring packet captures are _roughly live_
/// and any truncation from a crash is minimal.
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

impl ActorLifecycle for CaptureActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        ctx.set_interval(FLUSH_INTERVAL);
    }

    async fn on_tick(&mut self, _ctx: &mut DynContext<Self>) {
        self.flush_writers().await;
    }

    async fn on_shutdown(&mut self) {
        self.flush_writers().await;
    }
}
