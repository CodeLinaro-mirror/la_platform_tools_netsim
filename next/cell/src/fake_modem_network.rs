// Copyright 2024-2025 The Android Open Source Project
use bytes::Bytes;
use modem_rs::modem_network::{ModemCallbacks, ModemError, ModemNetworkInterface};
use netsim_model::cell::Cell;
use netsim_model::chip::{Chip, ChipId, ChipVariant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct ControllerState {
    callbacks: Arc<dyn ModemCallbacks>,
}

/// A Fake implementation of `ModemNetworkInterface` for testing `CellServer`.
///
/// This Fake is used to isolate tests within the `cell` crate from the
/// actual `modem-rs` implementation. It does not interact with any real modem
/// simulation logic. It primarily manages per-chip callbacks and provides
/// canned responses.
pub struct FakeModemNetwork {
    controllers: Mutex<HashMap<ChipId, ControllerState>>,
}

impl FakeModemNetwork {
    pub fn new() -> Arc<Self> {
        Arc::new(FakeModemNetwork { controllers: Mutex::new(HashMap::new()) })
    }
}

impl ModemNetworkInterface for FakeModemNetwork {
    fn add_modem(
        &self,
        chip_id: ChipId,
        callbacks: Arc<dyn ModemCallbacks>,
    ) -> Result<(), ModemError> {
        let mut controllers = self.controllers.lock().unwrap();
        if controllers.contains_key(&chip_id) {
            log::error!("[Fake] Modem {} already exists", chip_id);
            Err(ModemError::Internal(format!("Modem network {} already exists", chip_id)))
        } else {
            controllers.insert(chip_id, ControllerState { callbacks });
            log::info!("[Fake] Added modem: {}", chip_id);
            Ok(())
        }
    }

    fn remove_modem(&self, chip_id: ChipId) -> Result<(), ModemError> {
        let mut controllers = self.controllers.lock().unwrap();
        if controllers.remove(&chip_id).is_some() {
            log::info!("[Fake] Removed modem: {}", chip_id);
            Ok(())
        } else {
            log::warn!("[Fake] Modem {} not found for removal", chip_id);
            Err(ModemError::NotFound)
        }
    }

    fn send_data(&self, chip_id: ChipId, data: &[u8]) -> Result<(), ModemError> {
        log::debug!("[Fake] send_data for {}: {:?}", chip_id, data);
        let controllers = self.controllers.lock().unwrap();
        if let Some(state) = controllers.get(&chip_id) {
            // Simulate response: ECHO
            let response = Bytes::from(format!("ECHO: {}", String::from_utf8_lossy(data)));
            state.callbacks.on_data_received(response);
            Ok(())
        } else {
            Err(ModemError::NotFound)
        }
    }

    fn tick(&self) {
        // log::debug!("[Fake] CellularController tick");
    }

    fn get_modem_info(&self, chip_id: ChipId) -> Result<Chip, ModemError> {
        log::info!("[Fake] get_modem_info called for {}", chip_id);
        let controllers = self.controllers.lock().unwrap();
        log::info!("[Fake] Current controllers in map: {:?}", controllers.keys());
        if controllers.contains_key(&chip_id) {
            log::info!("[Fake] Controller {} FOUND", chip_id);
            let chip = Chip {
                variant: Some(ChipVariant::Cell(Cell { state: "FAKE_ACTIVE".to_string() })),
                ..Default::default()
            };
            Ok(chip)
        } else {
            log::warn!("[Fake] Controller {} NOT FOUND", chip_id);
            Err(ModemError::NotFound)
        }
    }
}
