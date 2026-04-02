// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use modem_rs::{test_utils::MockModemHandler, time::MockClock, ModemId, ModemNetworkSimulator};

/// The BDD World for Modem-rs tests.
#[allow(dead_code)]
pub struct World {
    /// The Modem Network Simulator under test.
    pub manager: ModemNetworkSimulator,
    /// Shared clock for the simulator.
    pub clock: Arc<MockClock>,
    /// Map of modem names to their IDs and handlers.
    pub modems: HashMap<String, (ModemId, MockModemHandler)>,
    /// Counter for generating unique ModemIds.
    pub modem_id_counter: usize,
}

impl World {
    pub fn new() -> Self {
        crate::common::init_logger();
        // No network handler needed for constructor.
        // If tests need to inspect network events, they should capture them from
        // dispatch output. For now, we assume tests rely on modem responses.
        let clock = Arc::new(MockClock::new());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let manager = ModemNetworkSimulator::new_with_clock(clock.clone(), tx);

        World { manager, clock, modems: HashMap::new(), modem_id_counter: 0 }
    }

    pub fn next_modem_id(&mut self) -> ModemId {
        self.modem_id_counter += 1;
        self.modem_id_counter as ModemId
    }

    /// Helper to get modem data by name
    pub fn get_modem(&mut self, name: &str) -> (ModemId, &mut MockModemHandler) {
        if let Some((id, handler)) = self.modems.get_mut(name) {
            (*id, handler)
        } else {
            panic!("Modem '{}' not found", name);
        }
    }
}
