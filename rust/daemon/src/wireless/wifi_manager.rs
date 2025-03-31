// Copyright 2025 Google LLC
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
use crate::get_runtime;
use crate::wifi::hostapd;
use crate::wifi::libslirp;
#[cfg(not(feature = "cuttlefish"))]
use crate::wifi::mdns_forwarder;
use crate::wifi::medium::Medium;
use crate::wireless::wifi_chip::{CreateParams, WifiChip};
use crate::wireless::{packet::handle_response, WirelessChipImpl};
use anyhow;
use bytes::Bytes;
use log::{info, warn};
use netsim_proto::config::WiFi as WiFiConfig;
use protobuf::MessageField;
use std::sync::{mpsc, Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tokio_mpsc;

/// Starts the WiFi service.
pub fn wifi_start(
    config: &MessageField<WiFiConfig>,
    forward_host_mdns: bool,
    wifi_args: Option<Vec<String>>,
    wifi_tap: Option<String>,
) {
    let (tx_request, rx_request) = mpsc::channel::<(u32, Bytes)>();
    let (tx_ieee8023_response, rx_ieee8023_response) = mpsc::channel::<Bytes>();
    let tx_ieee8023_response_clone = tx_ieee8023_response.clone();
    let wifi_config = config.clone().unwrap_or_default();

    let network: Box<dyn Network> = if wifi_tap.is_some() {
        todo!();
    } else {
        SlirpNetwork::start(config, tx_ieee8023_response_clone)
    };

    let hostapd_opt = wifi_config.hostapd_options.as_ref().unwrap_or_default().clone();
    // Create mpsc channel with fixed channel size
    let (tx_ieee80211_response, rx_ieee80211_response) = tokio_mpsc::channel(100);
    // Create the hostapd instance with global runtime
    let hostapd_result =
        get_runtime().block_on(hostapd::hostapd_run(hostapd_opt, tx_ieee80211_response, wifi_args));
    let hostapd = hostapd_result.map_err(|e| warn!("Failed to run hostapd. {e}")).unwrap();

    let _ = WIFI_MANAGER.set(Arc::new(WifiManager::new(tx_request, network, hostapd)));
    let wifi_manager = get_wifi_manager();

    if let Err(e) = start_threads(
        wifi_manager,
        rx_request,
        rx_ieee8023_response,
        rx_ieee80211_response,
        tx_ieee8023_response,
        forward_host_mdns,
    ) {
        warn!("Failed to start Wi-Fi manager: {}", e);
    }
}

/// Stops the WiFi service.
pub fn wifi_stop() {
    // TODO: stop hostapd
}

fn medium_callback(id: u32, packet: &Bytes) {
    handle_response(ChipIdentifier(id), packet);
}

/// Network interface for sending and receiving packets to/from the internet.
trait Network: Send + Sync {
    /// Sends the given bytes over the network to the internet.
    fn input(&self, bytes: Bytes);
}

/// A network implementation using libslirp.
struct SlirpNetwork {
    slirp: libslirp::LibSlirp,
}

impl SlirpNetwork {
    /// Starts a new SlirpNetwork instance.
    fn start(
        wifi_config: &WiFiConfig,
        tx_ieee8023_response: mpsc::Sender<Bytes>,
    ) -> Box<dyn Network> {
        let slirp_opt = wifi_config.slirp_options.as_ref().unwrap_or_default().clone();
        let slirp = libslirp::slirp_run(slirp_opt, tx_ieee8023_response)
            .map_err(|e| warn!("Failed to run libslirp. {e}"))
            .unwrap();
        Box::new(SlirpNetwork { slirp })
    }
}

impl Network for SlirpNetwork {
    fn input(&self, bytes: Bytes) {
        self.slirp.input(bytes);
    }
}

pub struct WifiManager {
    pub medium: Medium,
    pub tx_request: mpsc::Sender<(u32, Bytes)>,
    network: Box<dyn Network>,
    hostapd: Arc<hostapd::Hostapd>,
}

impl WifiManager {
    fn new(
        tx_request: mpsc::Sender<(u32, Bytes)>,
        network: Box<dyn Network>,
        hostapd: hostapd::Hostapd,
    ) -> WifiManager {
        let hostapd = Arc::new(hostapd);
        WifiManager {
            medium: Medium::new(medium_callback, hostapd.clone()),
            tx_request,
            network,
            hostapd,
        }
    }
}

/// Starts background threads:
/// * One to handle requests from medium.
/// * One to handle IEEE802.3 responses from network.
/// * One to handle IEEE802.11 responses from hostapd.
fn start_threads(
    wifi_manager: Arc<WifiManager>,
    rx_request: mpsc::Receiver<(u32, Bytes)>,
    rx_ieee8023_response: mpsc::Receiver<Bytes>,
    rx_ieee80211_response: tokio_mpsc::Receiver<Bytes>,
    tx_ieee8023_response: mpsc::Sender<Bytes>,
    forward_host_mdns: bool,
) -> anyhow::Result<()> {
    start_request_thread(wifi_manager.clone(), rx_request)?;
    start_ieee8023_response_thread(wifi_manager.clone(), rx_ieee8023_response)?;
    start_ieee80211_response_thread(wifi_manager.clone(), rx_ieee80211_response)?;
    if forward_host_mdns {
        start_mdns_forwarder_thread(tx_ieee8023_response)?;
    }
    Ok(())
}

fn start_request_thread(
    wifi_manager: Arc<WifiManager>,
    rx_request: mpsc::Receiver<(u32, Bytes)>,
) -> anyhow::Result<()> {
    let hostapd = wifi_manager.hostapd.clone(); // Arc clone for thread
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
                    if let Some(processor) = wifi_manager.medium.get_processor(chip_id, &packet) {
                        wifi_manager.medium.ack_frame(chip_id, &processor.frame);
                        if processor.hostapd {
                            let ieee80211: Bytes = processor.get_ieee80211_bytes();
                            let hostapd_clone = hostapd.clone();
                            get_runtime().block_on(async move {
                                if let Err(err) = hostapd_clone.input(ieee80211).await {
                                    warn!("Failed to call hostapd input: {:?}", err);
                                };
                            });
                        }
                        if processor.network {
                            match processor.get_ieee80211().to_ieee8023() {
                                Ok(ethernet_frame) => {
                                    wifi_manager.network.input(ethernet_frame.into())
                                }
                                Err(err) => {
                                    warn!("Failed to convert 802.11 to 802.3: {}", err)
                                }
                            }
                        }
                        if processor.wmedium {
                            // Decrypt the frame using the sender's key and re-encrypt it using the receiver's key for peer-to-peer communication through hostapd (broadcast or unicast).
                            let ieee80211 = processor.get_ieee80211().clone();
                            wifi_manager.medium.queue_frame(processor.frame, ieee80211);
                        }
                    }
                }
                _ => {
                    next_instant = Instant::now() + POLL_INTERVAL;
                }
            };
        }
    })?;
    Ok(())
}

/// Starts a dedicated thread to process IEEE 802.3 (Ethernet) responses from the network.
///
/// This thread continuously receives IEEE 802.3 response packets from the `rx_ieee8023_response` channel
/// and forwards them to the Wi-Fi manager's medium.
fn start_ieee8023_response_thread(
    wifi_manager: Arc<WifiManager>,
    rx_ieee8023_response: mpsc::Receiver<Bytes>,
) -> anyhow::Result<()> {
    thread::Builder::new().name("Wi-Fi IEEE802.3 response".to_string()).spawn(move || {
        for packet in rx_ieee8023_response {
            wifi_manager.medium.process_ieee8023_response(&packet);
        }
    })?;
    Ok(())
}

/// Starts a dedicated thread to process IEEE 802.11 responses from hostapd.
///
/// This thread continuously receives IEEE 802.11 response packets from the hostapd response channel
/// and forwards them to the Wi-Fi manager's medium.
fn start_ieee80211_response_thread(
    wifi_manager: Arc<WifiManager>,
    mut rx_ieee80211_response: tokio_mpsc::Receiver<Bytes>,
) -> anyhow::Result<()> {
    thread::Builder::new().name("Wi-Fi IEEE802.11 response".to_string()).spawn(move || {
        while let Some(packet) = get_runtime().block_on(rx_ieee80211_response.recv()) {
            wifi_manager.medium.process_ieee80211_response(&packet);
        }
    })?;
    Ok(())
}

#[cfg(feature = "cuttlefish")]
fn start_mdns_forwarder_thread(_tx_ieee8023_response: mpsc::Sender<Bytes>) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(not(feature = "cuttlefish"))]
fn start_mdns_forwarder_thread(tx_ieee8023_response: mpsc::Sender<Bytes>) -> anyhow::Result<()> {
    info!("Start mDNS forwarder thread");
    thread::Builder::new().name("Wi-Fi mDNS forwarder".to_string()).spawn(move || {
        if let Err(e) = mdns_forwarder::run_mdns_forwarder(tx_ieee8023_response) {
            warn!("Failed to start mDNS forwarder: {}", e);
        }
    })?;
    Ok(())
}

// Allocator for chip identifiers.
static WIFI_MANAGER: OnceLock<Arc<WifiManager>> = OnceLock::new();

fn get_wifi_manager() -> Arc<WifiManager> {
    WIFI_MANAGER.get().expect("WifiManager not initialized").clone()
}

/// Create a new Emulated Wifi Chip
/// allow(dead_code) due to not being used in unit tests
#[allow(dead_code)]
pub fn add_chip(_params: &CreateParams, chip_id: ChipIdentifier) -> WirelessChipImpl {
    let wifi_manager = get_wifi_manager();
    wifi_manager.medium.add(chip_id.0);
    info!("WiFi WirelessChip created chip_id: {chip_id}");
    let wifi = WifiChip { wifi_manager, chip_id };
    Box::new(wifi)
}
