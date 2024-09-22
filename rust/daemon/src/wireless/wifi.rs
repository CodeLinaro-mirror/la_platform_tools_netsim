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
use crate::wifi::hostapd;
use crate::wifi::libslirp;
#[cfg(not(feature = "cuttlefish"))]
use crate::wifi::mdns_forwarder;
use crate::wifi::medium::Medium;
use crate::wireless::{packet::handle_response, WirelessAdaptor, WirelessAdaptorImpl};
use anyhow;
use bytes::Bytes;
use log::{info, warn};
use netsim_proto::config::{HostapdOptions, SlirpOptions, WiFi as WiFiConfig};
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::{netsim_radio_stats, NetsimRadioStats as ProtoRadioStats};
use protobuf::{Message, MessageField};
use std::sync::atomic::Ordering;
use std::sync::{mpsc, OnceLock};
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
    tx_request: mpsc::Sender<(u32, Bytes)>,
    tx_response: mpsc::Sender<Bytes>,
    slirp: Option<libslirp::LibSlirp>,
    hostapd: Option<hostapd::Hostapd>,
}

impl WifiManager {
    pub fn new(
        tx_request: mpsc::Sender<(u32, Bytes)>,
        tx_response: mpsc::Sender<Bytes>,
        slirp: Option<libslirp::LibSlirp>,
        hostapd: Option<hostapd::Hostapd>,
    ) -> WifiManager {
        WifiManager {
            medium: Medium::new(medium_callback),
            tx_request,
            tx_response,
            slirp,
            hostapd,
        }
    }

    /// Starts background threads:
    /// * One to handle requests from medium.
    /// * One to handle responses from network.
    /// * One to handle IEEE802.3 responses from network.
    /// * One to handle IEEE802.11 responses from hostapd.
    pub fn start(
        &self,
        rx_request: mpsc::Receiver<(u32, Bytes)>,
        rx_response: mpsc::Receiver<Bytes>,
        rx_ieee8023_response: mpsc::Receiver<Bytes>,
        rx_ieee80211_response: mpsc::Receiver<Bytes>,
        tx_ieee8023_response: mpsc::Sender<Bytes>,
    ) -> anyhow::Result<()> {
        self.start_request_thread(rx_request)?;
        self.start_response_thread(rx_response)?;
        self.start_ieee8023_response_thread(rx_ieee8023_response)?;
        self.start_ieee80211_response_thread(rx_ieee80211_response)?;
        self.start_mdns_forwarder_thread(tx_ieee8023_response)?;
        Ok(())
    }

    fn start_request_thread(&self, rx_request: mpsc::Receiver<(u32, Bytes)>) -> anyhow::Result<()> {
        let rust_slirp = self.slirp.is_some();
        let rust_hostapd = self.hostapd.is_some();
        thread::Builder::new().name("Wi-Fi HwsimMsg request".to_string()).spawn(move || {
            const POLL_INTERVAL: Duration = Duration::from_millis(1);
            let mut next_instant = Instant::now() + POLL_INTERVAL;

            loop {
                let this_instant = Instant::now();
                let timeout = if next_instant > this_instant {
                    next_instant - this_instant
                } else {
                    Duration::ZERO
                };
                match rx_request.recv_timeout(timeout) {
                    Ok((chip_id, packet)) => {
                        if let Some(processor) =
                            get_wifi_manager().medium.get_processor(chip_id, &packet)
                        {
                            get_wifi_manager().medium.ack_frame(chip_id, &processor.frame);
                            if processor.hostapd {
                                if rust_hostapd {
                                    let ieee80211: Bytes = processor.frame.data.clone().into();
                                    get_wifi_manager()
                                        .hostapd
                                        .as_ref()
                                        .expect("hostapd initialized")
                                        .input(ieee80211);
                                } else {
                                    ffi_wifi::hostapd_send(&packet.to_vec());
                                }
                            }
                            if processor.network {
                                if rust_slirp {
                                    match processor.frame.ieee80211.to_ieee8023() {
                                        Ok(ethernet_frame) => get_wifi_manager()
                                            .slirp
                                            .as_ref()
                                            .expect("slirp initialized")
                                            .input(ethernet_frame.into()),
                                        Err(err) => {
                                            warn!("Failed to convert 802.11 to 802.3: {}", err)
                                        }
                                    }
                                } else {
                                    ffi_wifi::libslirp_send(&packet.to_vec());
                                    ffi_wifi::libslirp_main_loop_wait();
                                }
                            }
                            if processor.wmedium {
                                get_wifi_manager().medium.queue_frame(processor.frame);
                            }
                        }
                    }
                    _ => {
                        if !rust_slirp {
                            ffi_wifi::libslirp_main_loop_wait();
                        }
                        next_instant = Instant::now() + POLL_INTERVAL;
                    }
                };
            }
        })?;
        Ok(())
    }

    /// Starts a dedicated thread to handle WifiService responses.
    fn start_response_thread(&self, rx_response: mpsc::Receiver<Bytes>) -> anyhow::Result<()> {
        thread::Builder::new().name("WifiService response".to_string()).spawn(move || {
            for packet in rx_response {
                get_wifi_manager().medium.process_response(&packet);
            }
        })?;
        Ok(())
    }

    /// Starts a dedicated thread to process IEEE 802.3 (Ethernet) responses from the network.
    ///
    /// This thread continuously receives IEEE 802.3 response packets from the `rx_ieee8023_response` channel
    /// and forwards them to the Wi-Fi manager's medium.
    fn start_ieee8023_response_thread(
        &self,
        rx_ieee8023_response: mpsc::Receiver<Bytes>,
    ) -> anyhow::Result<()> {
        thread::Builder::new().name("Wi-Fi IEEE802.3 response".to_string()).spawn(move || {
            for packet in rx_ieee8023_response {
                get_wifi_manager().medium.process_ieee8023_response(&packet);
            }
        })?;
        Ok(())
    }

    /// Starts a dedicated thread to process IEEE 802.11 responses from hostapd.
    ///
    /// This thread continuously receives IEEE 802.11 response packets from the hostapd response channel
    /// and forwards them to the Wi-Fi manager's medium.
    fn start_ieee80211_response_thread(
        &self,
        rx_ieee80211_response: mpsc::Receiver<Bytes>,
    ) -> anyhow::Result<()> {
        thread::Builder::new().name("Wi-Fi IEEE802.11 response".to_string()).spawn(move || {
            for packet in rx_ieee80211_response {
                get_wifi_manager().medium.process_ieee80211_response(&packet);
            }
        })?;
        Ok(())
    }

    #[cfg(feature = "cuttlefish")]
    fn start_mdns_forwarder_thread(
        &self,
        _tx_ieee8023_response: mpsc::Sender<Bytes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(not(feature = "cuttlefish"))]
    fn start_mdns_forwarder_thread(
        &self,
        tx_ieee8023_response: mpsc::Sender<Bytes>,
    ) -> anyhow::Result<()> {
        info!("Start mDNS forwarder thread");
        thread::Builder::new().name("Wi-Fi mDNS forwarder".to_string()).spawn(move || {
            if let Err(e) = mdns_forwarder::run_mdns_forwarder(tx_ieee8023_response) {
                warn!("Failed to start mDNS forwarder: {}", e);
            }
        })?;
        Ok(())
    }
}

// Allocator for chip identifiers.
static WIFI_MANAGER: OnceLock<WifiManager> = OnceLock::new();

fn get_wifi_manager() -> &'static WifiManager {
    WIFI_MANAGER.get().expect("WifiManager not initialized")
}

impl Drop for Wifi {
    fn drop(&mut self) {
        get_wifi_manager().medium.remove(self.chip_id.0);
    }
}

impl WirelessAdaptor for Wifi {
    fn handle_request(&self, packet: &Bytes) {
        get_wifi_manager().tx_request.send((self.chip_id.0, packet.clone())).unwrap();
    }

    fn reset(&self) {
        get_wifi_manager().medium.reset(self.chip_id.0);
    }

    fn get(&self) -> ProtoChip {
        let mut chip_proto = ProtoChip::new();
        if let Some(client) = get_wifi_manager().medium.get(self.chip_id.0) {
            chip_proto.mut_wifi().state = Some(client.enabled.load(Ordering::Relaxed));
            chip_proto.mut_wifi().tx_count = client.tx_count.load(Ordering::Relaxed) as i32;
            chip_proto.mut_wifi().rx_count = client.rx_count.load(Ordering::Relaxed) as i32;
        }
        chip_proto
    }

    fn patch(&self, patch: &ProtoChip) {
        if patch.wifi().state.is_some() {
            get_wifi_manager().medium.set_enabled(self.chip_id.0, patch.wifi().state.unwrap());
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
    get_wifi_manager().tx_response.send(bytes).unwrap();
}

/// Create a new Emulated Wifi Chip
/// allow(dead_code) due to not being used in unit tests
#[allow(dead_code)]
pub fn new(_params: &CreateParams, chip_id: ChipIdentifier) -> WirelessAdaptorImpl {
    get_wifi_manager().medium.add(chip_id.0);
    info!("WiFi WirelessAdaptor created chip_id: {chip_id}");
    let wifi = Wifi { chip_id };
    Box::new(wifi)
}

/// Starts the WiFi service.
pub fn wifi_start(config: &MessageField<WiFiConfig>, rust_slirp: bool, rust_hostapd: bool) {
    let (tx_request, rx_request) = mpsc::channel::<(u32, Bytes)>();
    let (tx_response, rx_response) = mpsc::channel::<Bytes>();
    let (tx_ieee8023_response, rx_ieee8023_response) = mpsc::channel::<Bytes>();
    let (tx_ieee80211_response, rx_ieee80211_response) = mpsc::channel::<Bytes>();
    let tx_ieee8023_response_clone = tx_ieee8023_response.clone();
    let mut slirp = None;
    let mut wifi_config = config.clone().unwrap_or_default();
    if rust_slirp {
        let slirp_opt = wifi_config.slirp_options.as_ref().unwrap_or_default().clone();
        slirp = Some(
            libslirp::slirp_run(slirp_opt, tx_ieee8023_response_clone)
                .map_err(|e| warn!("Failed to run libslirp. {e}"))
                .unwrap(),
        );

        // Disable qemu slirp in WifiService
        wifi_config.slirp_options =
            MessageField::some(SlirpOptions { disabled: true, ..Default::default() });
    }

    let mut hostapd = None;
    if rust_hostapd {
        let hostapd_opt = wifi_config.hostapd_options.as_ref().unwrap_or_default().clone();
        hostapd = Some(
            hostapd::hostapd_run(hostapd_opt, tx_ieee80211_response)
                .map_err(|e| warn!("Failed to run hostapd. {e}"))
                .unwrap(),
        );

        // Disable qemu hostapd in WifiService
        wifi_config.hostapd_options =
            MessageField::some(HostapdOptions { disabled: Some(true), ..Default::default() });
    }

    let _ = WIFI_MANAGER.set(WifiManager::new(tx_request, tx_response, slirp, hostapd));

    // WifiService
    let proto_bytes = wifi_config.write_to_bytes().unwrap();
    ffi_wifi::wifi_start(&proto_bytes);

    if let Err(e) = get_wifi_manager().start(
        rx_request,
        rx_response,
        rx_ieee8023_response,
        rx_ieee80211_response,
        tx_ieee8023_response,
    ) {
        warn!("Failed to start Wi-Fi manager: {}", e);
    }
}

/// Stops the WiFi service.
pub fn wifi_stop() {
    // TODO: stop hostapd
    ffi_wifi::wifi_stop();
}
