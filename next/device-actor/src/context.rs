use netsim_model::capture::CaptureSender;
use netsim_model::chip::{ChipClient, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

pub struct DeviceContext {
    pub chip_clients: HashMap<NetworkKind, ChipClient>,
    pub next_chip_id: Arc<AtomicU32>,
    pub capture_client: Option<Arc<dyn CaptureSender>>, // Optional for now to avoid breaking tests
}
