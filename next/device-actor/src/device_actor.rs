use std::{
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc},
};

use actor_framework::TimerKey;
use capture_api::CaptureSender;
use netsim_model::{chip::ChipClient, ChipKind};

pub struct DeviceActor {
    pub chip_clients: HashMap<ChipKind, Box<dyn ChipClient>>,
    pub next_chip_id: Arc<AtomicU32>,
    pub capture_client: Option<Arc<dyn CaptureSender>>,
    pub(crate) devices: HashMap<device_api::DeviceId, crate::service::InternalDevice>,
    pub next_device_id: u32,
    pub link_client: Box<dyn link_api::LinkClient>,
    pub startup_timeout: Option<std::time::Duration>,
    pub idle_timeout: Option<std::time::Duration>,

    pub startup_timer: Option<TimerKey>,
    pub idle_timer: Option<TimerKey>,
    pub has_seen_device: bool,
    pub guid_to_id: HashMap<String, device_api::DeviceId>,
    pub stats_timer: Option<TimerKey>,
    pub stats_write_task: Option<tokio::task::JoinHandle<()>>,
    pub(crate) stats: crate::stats::Stats,
    pub stats_interval: std::time::Duration,
}

impl DeviceActor {
    pub fn new(
        chip_clients: HashMap<ChipKind, Box<dyn ChipClient>>,
        next_chip_id: Arc<AtomicU32>,
        capture_client: Option<Arc<dyn CaptureSender>>,
        link_client: Box<dyn link_api::LinkClient>,
        startup_timeout: Option<std::time::Duration>,
        idle_timeout: Option<std::time::Duration>,
        version: String,
        stats_path: Option<std::path::PathBuf>,
        stats_interval: Option<std::time::Duration>,
    ) -> Self {
        Self {
            chip_clients,
            next_chip_id,
            capture_client,
            devices: HashMap::new(),
            next_device_id: 1,
            link_client,
            startup_timeout,
            idle_timeout,

            startup_timer: None,
            idle_timer: None,
            has_seen_device: false,
            guid_to_id: HashMap::new(),
            stats_timer: None,
            stats_write_task: None,
            stats: crate::stats::Stats::new(version, stats_path),
            stats_interval: stats_interval.unwrap_or(std::time::Duration::from_secs(10)),
        }
    }

    pub(crate) fn schedule_periodic_stats(&mut self, ctx: &mut dyn actor_framework::Context<Self>) {
        let base_stats = self.stats.get_base_stats();
        let path = self.stats.stats_path.clone();
        let interval = self.stats_interval;

        // Verify previous background write task finished before spawning another
        if let Some(task) = &self.stats_write_task {
            if !task.is_finished() {
                log::warn!(
                    "DeviceActor: Dropping periodic stats tick; previous write is still pending."
                );
                return;
            }
        }

        self.stats_write_task = Some(tokio::task::spawn_blocking(move || {
            if let Err(e) = crate::stats::write_combined_stats(base_stats, path) {
                log::error!("DeviceActor: {}", e);
            }
        }));

        let key = ctx.run_later(
            interval,
            Box::new(move |actor, ctx| {
                actor.schedule_periodic_stats(ctx);
            }),
        );
        self.stats_timer = Some(key);
    }
}

impl std::fmt::Debug for DeviceActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceActor")
            .field("next_chip_id", &self.next_chip_id)
            .field("devices", &self.devices)
            .field("startup_timeout", &self.startup_timeout)
            .field("idle_timeout", &self.idle_timeout)
            .field("startup_timer", &self.startup_timer)
            .field("idle_timer", &self.idle_timer)
            .field("has_seen_device", &self.has_seen_device)
            .field("stats_interval", &self.stats_interval)
            .finish_non_exhaustive()
    }
}
