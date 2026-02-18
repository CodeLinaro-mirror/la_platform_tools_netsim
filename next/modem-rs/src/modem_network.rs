// modem-rs/src/controller.rs
use std::{fmt, sync::Arc};

use bytes::Bytes;
use netsim_model::chip::{Chip, ChipId};

#[derive(Debug, Clone)]
pub enum ModemError {
    Internal(String),
    NotFound,
}

impl std::error::Error for ModemError {}

impl fmt::Display for ModemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModemError::NotFound => write!(f, "Modem network not found"),
            ModemError::Internal(s) => write!(f, "Modem network internal error: {}", s),
        }
    }
}

// Callbacks that the CellularController will call into for events for a
// specific ChipId
pub trait ModemCallbacks: Send + Sync {
    fn on_data_received(&self, data: Bytes);
    fn on_event(&self, event: String);
}

pub trait ModemNetworkInterface: Send + Sync {
    fn add_modem(
        &self,
        chip_id: ChipId,
        callbacks: Arc<dyn ModemCallbacks>,
    ) -> Result<(), ModemError>;
    fn remove_modem(&self, chip_id: ChipId) -> Result<(), ModemError>;
    fn send_data(&self, chip_id: ChipId, data: &[u8]) -> Result<(), ModemError>;
    fn tick(&self);
    fn get_modem_info(&self, chip_id: ChipId) -> Result<Chip, ModemError>;
}
