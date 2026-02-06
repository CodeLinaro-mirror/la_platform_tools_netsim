use capture_api::CaptureSender;
use netsim_model::chip::ChipClient;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

pub struct DeviceActor {
    pub chip_clients: HashMap<netsim_model::chip::ChipKind, Box<dyn ChipClient>>,
    pub next_chip_id: Arc<AtomicU32>,
    pub capture_client: Option<Arc<dyn CaptureSender>>,
    pub(crate) devices: HashMap<device_api::DeviceId, crate::service::InternalDevice>,
    pub next_device_id: u32,
    pub link_client: Box<dyn link_api::LinkClient>,
    pub startup_timeout: Option<std::time::Duration>,
    pub idle_timeout: Option<std::time::Duration>,
    pub start_time: std::time::Instant,
    pub last_empty_time: Option<std::time::Instant>,
    pub has_seen_device: bool,
    pub guid_to_id: HashMap<String, device_api::DeviceId>,
}

impl DeviceActor {
    pub fn new(
        chip_clients: HashMap<netsim_model::chip::ChipKind, Box<dyn ChipClient>>,
        next_chip_id: Arc<AtomicU32>,
        capture_client: Option<Arc<dyn CaptureSender>>,
        link_client: Box<dyn link_api::LinkClient>,
        startup_timeout: Option<std::time::Duration>,
        idle_timeout: Option<std::time::Duration>,
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
            start_time: std::time::Instant::now(),
            last_empty_time: Some(std::time::Instant::now()),
            has_seen_device: false,
            guid_to_id: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for DeviceActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceActor")
            .field("next_chip_id", &self.next_chip_id)
            .field("devices", &self.devices)
            .field("startup_timeout", &self.startup_timeout)
            .field("idle_timeout", &self.idle_timeout)
            .field("start_time", &self.start_time)
            .field("last_empty_time", &self.last_empty_time)
            .field("has_seen_device", &self.has_seen_device)
            .finish_non_exhaustive()
    }
}
