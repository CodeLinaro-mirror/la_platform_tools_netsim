use actor_framework::ActorContext;
use async_trait::async_trait;
use capture_api::CaptureSender;
use netsim_model::chip::{ChipClient, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

pub struct DeviceContext {
    pub chip_clients: HashMap<NetworkKind, ChipClient>,
    pub next_chip_id: Arc<AtomicU32>,
    pub capture_client: Option<Arc<dyn CaptureSender>>, // Optional for now to avoid breaking tests
                                                        //TODO: Add link_client
}

#[async_trait]
impl ActorContext for DeviceContext {
    type Error = crate::error::DeviceError;
}
