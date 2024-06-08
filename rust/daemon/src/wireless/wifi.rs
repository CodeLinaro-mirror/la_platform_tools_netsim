// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::devices::chip::ChipIdentifier;
use crate::ffi::ffi_wifi;
use crate::wifi::medium::Medium;
use crate::wireless::{packet::handle_response, WirelessAdaptor, WirelessAdaptorImpl};
use bytes::Bytes;
use lazy_static::lazy_static;
use log::info;
use netsim_proto::config::WiFi as WiFiConfig;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::{netsim_radio_stats, NetsimRadioStats as ProtoRadioStats};
use protobuf::{Message, MessageField};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

/// Parameters for creating Wifi chips
/// allow(dead_code) due to not being used in unit tests
#[allow(dead_code)]
pub struct CreateParams {}

/// Wifi struct will keep track of chip_id
pub struct Wifi {
    chip_id: ChipIdentifier,
}

pub struct WifiManager {
    medium: Medium,
    request_sender: std::sync::mpsc::Sender<(u32, Bytes)>,
    response_sender: std::sync::mpsc::Sender<Bytes>,
}

impl WifiManager {
    pub fn new() -> WifiManager {
        let (request_sender, rx) = std::sync::mpsc::channel::<(u32, Bytes)>();

        thread::spawn(move || {
            const POLL_INTERVAL: Duration = Duration::from_millis(1);
            let mut next_instant = Instant::now() + POLL_INTERVAL;

            loop {
                let this_instant = Instant::now();
                let timeout = if next_instant > this_instant {
                    next_instant - this_instant
                } else {
                    Duration::ZERO
                };
                match rx.recv_timeout(timeout) {
                    Ok((chip_id, packet)) => {
                        // When Wi-Fi P2P is disabled, send all packets to WifiService.
                        if crate::config::get_disable_wifi_p2p()
                            || !WIFI_MANAGER.medium.process(chip_id, &packet)
                        {
                            ffi_wifi::handle_wifi_request(chip_id, &packet.to_vec());
                            ffi_wifi::libslirp_main_loop_wait();
                        }
                    }
                    _ => {
                        ffi_wifi::libslirp_main_loop_wait();
                        next_instant = Instant::now() + POLL_INTERVAL;
                    }
                };
            }
        });

        let (response_sender, rx) = std::sync::mpsc::channel::<Bytes>();
        thread::spawn(move || loop {
            let packet = rx.recv().unwrap();
            WIFI_MANAGER.medium.process_response(&packet);
        });
        WifiManager { medium: Medium::new(medium_callback), request_sender, response_sender }
    }
}

// Allocator for chip identifiers.
lazy_static! {
    static ref WIFI_MANAGER: WifiManager = WifiManager::new();
}

impl Drop for Wifi {
    fn drop(&mut self) {
        WIFI_MANAGER.medium.remove(self.chip_id.0);
    }
}

impl WirelessAdaptor for Wifi {
    fn handle_request(&self, packet: &Bytes) {
        WIFI_MANAGER.request_sender.send((self.chip_id.0, packet.clone())).unwrap();
    }

    fn reset(&self) {
        WIFI_MANAGER.medium.reset(self.chip_id.0);
    }

    fn get(&self) -> ProtoChip {
        let mut chip_proto = ProtoChip::new();
        if let Some(client) = WIFI_MANAGER.medium.get(self.chip_id.0) {
            chip_proto.mut_wifi().state = Some(client.enabled.load(Ordering::Relaxed));
            chip_proto.mut_wifi().tx_count = client.tx_count.load(Ordering::Relaxed) as i32;
            chip_proto.mut_wifi().rx_count = client.rx_count.load(Ordering::Relaxed) as i32;
        }
        chip_proto
    }

    fn patch(&self, patch: &ProtoChip) {
        if patch.wifi().state.is_some() {
            WIFI_MANAGER.medium.set_enabled(self.chip_id.0, patch.wifi().state.unwrap());
        }
    }

    fn get_stats(&self, duration_secs: u64) -> Vec<ProtoRadioStats> {
        let mut stats_proto = ProtoRadioStats::new();
        stats_proto.set_duration_secs(duration_secs);
        stats_proto.set_kind(netsim_radio_stats::Kind::WIFI);
        let chip_proto = self.get();
        if chip_proto.has_wifi() {
            stats_proto.set_tx_count(chip_proto.wifi().tx_count);
            stats_proto.set_rx_count(chip_proto.wifi().rx_count);
        }
        vec![stats_proto]
    }
}

fn medium_callback(id: u32, packet: &Bytes) {
    handle_response(ChipIdentifier(id), packet);
}

pub fn handle_wifi_response(packet: &[u8]) {
    let bytes = Bytes::copy_from_slice(packet);
    WIFI_MANAGER.response_sender.send(bytes).unwrap();
}

/// Create a new Emulated Wifi Chip
/// allow(dead_code) due to not being used in unit tests
#[allow(dead_code)]
pub fn new(_params: &CreateParams, chip_id: ChipIdentifier) -> WirelessAdaptorImpl {
    WIFI_MANAGER.medium.add(chip_id.0);
    info!("WiFi WirelessAdaptor created chip_id: {chip_id}");
    let wifi = Wifi { chip_id };
    Box::new(wifi)
}

/// Starts the WiFi service.
pub fn wifi_start(config: &MessageField<WiFiConfig>) {
    let proto_bytes = config.as_ref().unwrap_or_default().write_to_bytes().unwrap();
    ffi_wifi::wifi_start(&proto_bytes);
}

/// Stops the WiFi service.
pub fn wifi_stop() {
    ffi_wifi::wifi_stop();
}
