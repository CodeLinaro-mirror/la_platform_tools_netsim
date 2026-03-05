// tests/common/mod.rs

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use log;
use modem_rs::{time::MockClock, *};

static LOG_INIT: std::sync::Once = std::sync::Once::new();

#[allow(dead_code)]
pub fn init_logger() {
    LOG_INIT.call_once(|| {
        env_logger::init();
    });
}

#[cfg(test)]
#[allow(dead_code)]
pub mod constants;

// Mock handler for network callbacks
pub struct MockNetworkHandler;
impl NetworkCallbacks for MockNetworkHandler {
    fn on_new_remote_connection(&self, _modem_id: ModemId, _destination: String) {}
    fn on_modem_hanged_up(&self, _modem_id: ModemId) {}
}

// Mock handler for modem callbacks that captures the responses.
#[allow(dead_code)]
pub struct MockModemHandler {
    pub responses: Mutex<Vec<Vec<u8>>>,
}
impl Callbacks for MockModemHandler {
    fn send_at_response(&self, _modem_id: ModemId, response: &[u8]) {
        log::debug!(
            "[MockModemHandler] Capturing response: {:?}",
            String::from_utf8_lossy(response)
        );
        self.responses.lock().unwrap().push(response.to_vec());
    }
}

#[allow(dead_code)]
impl MockModemHandler {
    pub fn new() -> Self {
        Self { responses: Mutex::new(Vec::new()) }
    }

    pub fn get_responses(&self) -> Vec<Vec<u8>> {
        self.responses.lock().unwrap().drain(..).collect()
    }
}

#[allow(dead_code)]
pub struct TestHarness {
    pub manager: Arc<ModemNetworkSimulator>,
    pub modem_id: ModemId,
    pub modem_handler: Arc<MockModemHandler>,
    pub clock: Arc<MockClock>,
    modem_handlers: HashMap<ModemId, Arc<MockModemHandler>>,
}

#[allow(dead_code)]
impl TestHarness {
    pub fn new() -> Self {
        let profile = modem_rs::config::SimProfile {
            iccid: "89012345678901234567".to_string(),
            imsi: "123456789012345".to_string(),
            sim_io: modem_rs::config::SimIo {
                file_system: modem_rs::config::FileSystem {
                    master_file: modem_rs::config::DedicatedFile {
                        file_id: "3F00".to_string(),
                        files: vec![modem_rs::config::SimFile::Ef(
                            modem_rs::config::ElementaryFile {
                                file_id: "2FE2".to_string(),
                                size: 10,
                                data: "89012345678901234567".to_string(),
                            },
                        )],
                    },
                },
            },
            ..Default::default()
        };
        Self::new_with_sim_profile(Some(profile))
    }

    pub fn new_with_sim_profile(profile: Option<modem_rs::config::SimProfile>) -> Self {
        let network_handler = Arc::new(MockNetworkHandler);
        let clock = Arc::new(MockClock::new());
        let manager = ModemNetworkSimulator::new_with_clock(network_handler, clock.clone());
        let mut modem_handlers = HashMap::new();

        let modem_id: ModemId = 1;
        let modem_handler = Arc::new(MockModemHandler::new());
        manager.new_modem_with_profile(modem_id, modem_handler.clone(), profile).unwrap();
        modem_handlers.insert(modem_id, modem_handler.clone());

        // Add a second modem for SMS tests
        let modem2_id: ModemId = 2;
        let modem2_handler = Arc::new(MockModemHandler::new());
        manager.new_modem(modem2_id, modem2_handler.clone()).unwrap();
        modem_handlers.insert(modem2_id, modem2_handler.clone());
        if let Some(modem) = manager.get_modem(modem_id) {
            modem.set_phone_number(constants::PHONE_NUMBER_C);
        }
        if let Some(modem) = manager.get_modem(modem2_id) {
            modem.set_phone_number(constants::PHONE_NUMBER_D);
        }

        // Add a third modem for multi-call tests
        let modem3_id: ModemId = 3;
        let modem3_handler = Arc::new(MockModemHandler::new());
        manager.new_modem(modem3_id, modem3_handler.clone()).unwrap();
        modem_handlers.insert(modem3_id, modem3_handler.clone());
        if let Some(modem) = manager.get_modem(modem3_id) {
            modem.set_phone_number("222");
        }

        Self { manager, modem_id, modem_handler, clock, modem_handlers }
    }

    pub fn send_at_command(&self, command: &[u8]) {
        self.manager.send_at_command(self.modem_id, command);
    }

    pub fn get_responses(&self) -> Vec<Vec<u8>> {
        self.modem_handler.get_responses()
    }

    pub fn get_modem_handler(&self, modem_id: ModemId) -> Option<Arc<MockModemHandler>> {
        self.modem_handlers.get(&modem_id).cloned()
    }
}
