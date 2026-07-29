// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use ap_actor::SharedKeyStore;
use netsim_packets::{HwsimFrame, MacAddress};
use tracing::{info, warn};

use crate::{
    DebugArgs,
    error::WifiError,
    medium::types::{Client, Station, WifiResult},
    stats::WifiStats,
};

pub struct Medium {
    pub stations: HashMap<MacAddress, Station>,
    pub clients: HashMap<u32, Client>,
    /// If true, the Medium reflects `ToDS` frames as `FromDS` frames when
    /// deliverying to other stations. This allows peer-to-peer
    /// communication via the AP without passing through a full AP stack.
    pub(crate) simulate_ap_reflection: bool,
    pub(crate) key_store: Arc<SharedKeyStore>,
    pub wifi_stats: WifiStats,
    pub(crate) debug: Arc<DebugArgs>,
    pub(crate) seq: std::sync::atomic::AtomicU16,
}

impl std::fmt::Debug for Medium {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Medium")
            .field("stations", &self.stations)
            .field("clients", &self.clients)
            .field("simulate_ap_reflection", &self.simulate_ap_reflection)
            .field("key_store", &self.key_store)
            .field("wifi_stats", &self.wifi_stats)
            .field("debug", &self.debug)
            .finish()
    }
}

impl Medium {
    // Data Flow Documentation:
    // ...
    pub fn new(
        key_store: Arc<SharedKeyStore>,
        wifi_stats: WifiStats,
        debug: Arc<DebugArgs>,
    ) -> Medium {
        Self {
            stations: HashMap::new(),
            clients: HashMap::new(),
            simulate_ap_reflection: true,
            key_store,
            wifi_stats,
            debug,
            seq: std::sync::atomic::AtomicU16::new(100),
        }
    }

    pub fn add(&mut self, client_id: u32) {
        let _ = self.clients.entry(client_id).or_insert_with(|| {
            info!("Insert client {client_id}");
            Client::new()
        });
    }

    pub fn remove(&mut self, client_id: u32) {
        self.stations.retain(|_, s| s.client_id != client_id);
        self.clients.remove(&client_id);
    }

    pub fn reset(&mut self, client_id: u32) {
        self.stations.retain(|_, s| s.client_id != client_id);
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.enabled = true;
            client.tx_count = 0;
            client.rx_count = 0;
            client.p2p_tx_count = 0;
            client.p2p_rx_count = 0;
        }
    }

    pub fn contains_client(&self, client_id: u32) -> bool {
        self.clients.contains_key(&client_id)
    }

    pub fn contains_station(&self, addr: &MacAddress) -> bool {
        self.stations.contains_key(addr)
    }

    pub fn get_station_chip_id(&self, addr: &MacAddress) -> Option<u32> {
        self.stations.get(addr).map(|s| s.client_id)
    }

    pub(crate) fn get_station(&self, addr: &MacAddress) -> WifiResult<&Station> {
        self.stations.get(addr).ok_or_else(|| {
            WifiError::Internal(Box::from(format!("Station not found for address: {addr}")))
        })
    }

    pub(crate) fn get_station_mut(&mut self, addr: &MacAddress) -> WifiResult<&mut Station> {
        self.stations.get_mut(addr).ok_or_else(|| {
            WifiError::Internal(Box::from(format!("Station not found for address: {addr}")))
        })
    }

    pub(crate) fn upsert_station(&mut self, client_id: u32, frame: &HwsimFrame) -> WifiResult<()> {
        let src_addr = frame.ieee80211.get_source();
        let hwsim_addr = frame.transmitter.ok_or(WifiError::Internal(Box::from(format!(
            "Missing transmitter attribute in frame for client: {client_id}"
        ))))?;
        self.stations.entry(src_addr).or_insert_with(|| {
            info!(
                "Insert station with client id {client_id}, hwsimaddr: {hwsim_addr}, \
                Ieee80211 addr: {src_addr}"
            );
            Station::new(client_id, src_addr, hwsim_addr)
        });
        if !self.contains_client(client_id) {
            warn!("Client {client_id} is missing");
            self.add(client_id);
        }
        Ok(())
    }

    pub fn set_enabled(&mut self, client_id: u32, enabled: bool) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.enabled = enabled;
        }
    }

    pub(crate) fn enabled(&self, client_id: u32) -> WifiResult<bool> {
        self.clients
            .get(&client_id)
            .map(|c| c.enabled)
            .ok_or_else(|| WifiError::Internal(Box::from(format!("client {client_id} is missing"))))
    }

    pub(crate) fn incr_tx(&mut self, client_id: u32) -> WifiResult<()> {
        self.clients.get_mut(&client_id).map_or(
            Err(WifiError::Internal(Box::from(format!(
                "client {client_id} is missing for incr_tx"
            )))),
            |c| {
                c.tx_count += 1;
                Ok(())
            },
        )
    }

    pub(crate) fn incr_rx(&mut self, client_id: u32) -> WifiResult<()> {
        self.clients.get_mut(&client_id).map_or(
            Err(WifiError::Internal(Box::from(format!(
                "client {client_id} is missing for incr_rx"
            )))),
            |c| {
                c.rx_count += 1;
                Ok(())
            },
        )
    }

    pub fn get_tx_count(&self, client_id: u32) -> u32 {
        self.clients.get(&client_id).map(|c| c.tx_count).unwrap_or(0)
    }

    pub fn get_rx_count(&self, client_id: u32) -> u32 {
        self.clients.get(&client_id).map(|c| c.rx_count).unwrap_or(0)
    }

    pub(crate) fn incr_p2p_tx(&mut self, client_id: u32) {
        if let Some(c) = self.clients.get_mut(&client_id) {
            c.p2p_tx_count += 1;
        } else {
            warn!("client {client_id} is missing for incr_p2p_tx");
        }
    }

    pub(crate) fn incr_p2p_rx(&mut self, client_id: u32) {
        if let Some(c) = self.clients.get_mut(&client_id) {
            c.p2p_rx_count += 1;
        } else {
            warn!("client {client_id} is missing for incr_p2p_rx");
        }
    }

    pub fn get_p2p_tx_count(&self, client_id: u32) -> u64 {
        self.clients.get(&client_id).map(|c| c.p2p_tx_count).unwrap_or(0)
    }

    pub fn get_p2p_rx_count(&self, client_id: u32) -> u64 {
        self.clients.get(&client_id).map(|c| c.p2p_rx_count).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_telemetry_counters() {
        use std::sync::Arc;

        use ap_actor::SharedKeyStore;

        use crate::{
            DebugArgs,
            stats::{SystemClock, WifiStats},
        };

        let key_store = Arc::new(SharedKeyStore::new());
        let clock = Arc::new(SystemClock);
        let wifi_stats = WifiStats::new(clock);
        let debug = Arc::new(DebugArgs::default());

        let mut medium = Medium::new(key_store, wifi_stats, debug);
        let client_id = 1;
        medium.add(client_id);

        assert_eq!(medium.get_p2p_tx_count(client_id), 0);
        assert_eq!(medium.get_p2p_rx_count(client_id), 0);

        medium.incr_p2p_tx(client_id);
        assert_eq!(medium.get_p2p_tx_count(client_id), 1);
        assert_eq!(medium.get_p2p_rx_count(client_id), 0);

        medium.incr_p2p_rx(client_id);
        assert_eq!(medium.get_p2p_tx_count(client_id), 1);
        assert_eq!(medium.get_p2p_rx_count(client_id), 1);

        medium.reset(client_id);
        assert_eq!(medium.get_p2p_tx_count(client_id), 0);
        assert_eq!(medium.get_p2p_rx_count(client_id), 0);
    }
}
