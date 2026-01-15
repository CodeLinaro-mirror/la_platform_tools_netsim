use capture_api::CaptureSender;
use netsim_model::chip::{ChipClient, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

pub struct DeviceActor {
    pub chip_clients: HashMap<NetworkKind, Box<dyn ChipClient>>,
    pub next_chip_id: Arc<AtomicU32>,
    pub capture_client: Option<Arc<dyn CaptureSender>>,
    pub(crate) devices: HashMap<device_api::DeviceId, crate::service::InternalDevice>,
    pub next_device_id: u32,
    pub link_client: Box<dyn link_api::LinkClient>,
}

impl DeviceActor {
    pub fn new(
        chip_clients: HashMap<NetworkKind, Box<dyn ChipClient>>,
        next_chip_id: Arc<AtomicU32>,
        capture_client: Option<Arc<dyn CaptureSender>>,
        link_client: Box<dyn link_api::LinkClient>,
    ) -> Self {
        Self {
            chip_clients,
            next_chip_id,
            capture_client,
            devices: HashMap::new(),
            next_device_id: 1,
            link_client,
        }
    }
}

impl std::fmt::Debug for DeviceActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceActor")
            .field("next_chip_id", &self.next_chip_id)
            .field("devices", &self.devices)
            .finish_non_exhaustive()
    }
}
