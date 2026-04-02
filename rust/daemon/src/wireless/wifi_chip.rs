// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::devices::chip::ChipIdentifier;
use crate::wireless::wifi_manager::WifiManager;
use crate::wireless::WirelessChip;
use bytes::Bytes;
use log::warn;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::{netsim_radio_stats, NetsimRadioStats as ProtoRadioStats};
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Parameters for creating WifiChips
/// allow(dead_code) due to not being used in unit tests
#[allow(dead_code)]
pub struct CreateParams {}

/// WifiChip struct will keep track of chip_id
pub struct WifiChip {
    pub chip_id: ChipIdentifier,
    pub wifi_manager: Arc<WifiManager>,
}

impl Drop for WifiChip {
    fn drop(&mut self) {
        self.wifi_manager.medium.remove(self.chip_id.0);
    }
}

impl WirelessChip for WifiChip {
    fn handle_request(&self, packet: &Bytes) {
        if let Err(e) = self.wifi_manager.tx_request.send((self.chip_id.0, packet.clone())) {
            warn!("Failed wifi handle_request: {e:?}");
        }
    }

    fn reset(&self) {
        self.wifi_manager.medium.reset(self.chip_id.0);
    }

    fn get(&self) -> ProtoChip {
        let mut chip_proto = ProtoChip::new();
        if let Some(client) = self.wifi_manager.medium.get(self.chip_id.0) {
            chip_proto.mut_wifi().state = Some(client.enabled.load(Ordering::Relaxed));
            chip_proto.mut_wifi().tx_count = client.tx_count.load(Ordering::Relaxed) as i32;
            chip_proto.mut_wifi().rx_count = client.rx_count.load(Ordering::Relaxed) as i32;
        }
        chip_proto
    }

    fn patch(&self, patch: &ProtoChip) {
        if patch.wifi().state.is_some() {
            self.wifi_manager.medium.set_enabled(self.chip_id.0, patch.wifi().state.unwrap());
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
