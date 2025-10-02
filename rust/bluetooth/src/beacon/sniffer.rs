// Copyright (C) 2025 The Android Open Source Project

//! A BLE sniffer for testing purposes.

use rootcanal::bluetooth::Bluetooth;
use rootcanal::controller::{Callbacks as ControllerCallbacks, Id, Idc};
use rootcanal::types::{Address, Phy};
use std::ffi::c_int;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct SnifferCallbacks {
    packets: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl ControllerCallbacks for SnifferCallbacks {
    fn send_hci(&self, _source_id: Id, idc: Idc, data: &[u8]) {
        if idc == Idc::Evt {
            self.packets.lock().unwrap().push(data.to_vec());
        }
    }
    fn on_receive_ll(&self, _source_id: Id, _packet: &[u8], _phy: Phy, _tx_power: i32) {}
    fn invalid_packet_received(
        &self,
        _source_id: Id,
        _reason: c_int,
        _message: &str,
        _data: &[u8],
    ) {
    }
}

/// A BLE sniffer that can receive advertisements and send scan requests.
#[allow(dead_code)]
pub struct Sniffer {
    rootcanal: Arc<Bluetooth>,
    pub id: u32,
    callbacks: SnifferCallbacks,
}

#[allow(dead_code)]
impl Sniffer {
    /// Creates a new sniffer.
    pub fn new(rootcanal: Arc<Bluetooth>, address: &str) -> Self {
        let packets = Arc::new(Mutex::new(Vec::new()));
        let callbacks = SnifferCallbacks { packets };
        let address = Address::from_str(address).unwrap();
        let id = rootcanal.new_controller(address, Box::new(callbacks.clone()));
        Self { rootcanal, id, callbacks }
    }

    pub fn get_packets(&self) -> Vec<Vec<u8>> {
        self.callbacks.packets.lock().unwrap().clone()
    }
}

impl Drop for Sniffer {
    fn drop(&mut self) {
        let _ = self.rootcanal.remove_controller(self.id);
    }
}
