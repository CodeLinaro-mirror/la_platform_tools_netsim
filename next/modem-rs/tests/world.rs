// Copyright 2025 The Android Open Source Project

use std::{collections::HashMap, sync::Arc};

use modem_rs::{
    test_utils::{MockModemHandler, MockNetworkHandler},
    time::MockClock,
    types::ModemId,
    ModemNetworkSimulator,
};

/// The BDD World for Modem-rs tests.
#[allow(dead_code)]
pub struct World {
    /// The Modem Network Simulator under test.
    pub manager: Arc<ModemNetworkSimulator>,
    /// Shared clock for the simulator.
    pub clock: Arc<MockClock>,
    /// Map of modem names to their IDs and handlers.
    pub modems: HashMap<String, (ModemId, Arc<MockModemHandler>)>,
    /// Counter for generating unique ModemIds.
    pub modem_id_counter: usize,
}

impl World {
    pub fn new() -> Self {
        crate::common::init_logger();
        let network_handler = Arc::new(MockNetworkHandler::new());
        let clock = Arc::new(MockClock::new());
        let manager = ModemNetworkSimulator::new_with_clock(network_handler, clock.clone());

        World { manager, clock, modems: HashMap::new(), modem_id_counter: 0 }
    }

    pub fn next_modem_id(&mut self) -> ModemId {
        self.modem_id_counter += 1;
        self.modem_id_counter as ModemId
    }

    /// Helper to get modem data by name
    pub fn get_modem(&self, name: &str) -> (ModemId, Arc<MockModemHandler>) {
        self.modems.get(name).cloned().expect(&format!("Modem '{}' not found", name))
    }
}
