// src/test_utils.rs

use std::{
    sync::{Condvar, Mutex},
    time::Duration,
};

use crate::*;

/// Mock handler for manager callbacks that captures events.
pub struct MockNetworkHandler {
    remote_conns: Mutex<Vec<(ModemId, String)>>,
    cvar: Condvar,
}

impl NetworkCallbacks for MockNetworkHandler {
    fn on_new_remote_connection(&self, modem_id: ModemId, destination: String) {
        let mut conns = self.remote_conns.lock().unwrap();
        conns.push((modem_id, destination));
        self.cvar.notify_one();
    }

    fn on_modem_hanged_up(&self, _modem_id: ModemId) {
        // For now, do nothing. This can be expanded if tests need to assert on
        // this event.
    }
}

impl MockNetworkHandler {
    pub fn new() -> Self {
        Self { remote_conns: Mutex::new(Vec::new()), cvar: Condvar::new() }
    }

    pub fn wait_for_remote_connection(&self) -> Option<(ModemId, String)> {
        let mut conns = self.remote_conns.lock().unwrap();
        if conns.is_empty() {
            let result = self.cvar.wait_timeout(conns, Duration::from_secs(1)).unwrap();
            conns = result.0;
            if result.1.timed_out() {
                return None;
            }
        }
        Some(conns.remove(0))
    }
}

/// Mock handler for modem callbacks that captures responses and allows waiting.
pub struct MockModemHandler {
    responses: Mutex<Vec<Vec<u8>>>,
    cvar: Condvar,
}

impl Callbacks for MockModemHandler {
    fn send_at_response(&self, modem_id: ModemId, response: &[u8]) {
        // TODO: Remove this log
        println!("[Test] Modem {} received: {:?}", modem_id, String::from_utf8_lossy(response));
        let mut responses = self.responses.lock().unwrap();
        responses.push(response.to_vec());
        self.cvar.notify_one();
    }
}

impl MockModemHandler {
    pub fn new() -> Self {
        Self { responses: Mutex::new(Vec::new()), cvar: Condvar::new() }
    }

    /// Waits for a response for a specific duration and returns the oldest one.
    pub fn wait_for_response_with_timeout(&self, timeout: Duration) -> Option<Vec<u8>> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            let result = self.cvar.wait_timeout(responses, timeout).unwrap();
            responses = result.0;
            if result.1.timed_out() {
                return None;
            }
        }
        Some(responses.remove(0))
    }

    /// Waits for a response to be available and returns the oldest one.
    pub fn wait_for_response(&self) -> Vec<u8> {
        self.wait_for_response_with_timeout(Duration::from_secs(1))
            .expect("Test timed out waiting for response")
    }
}
