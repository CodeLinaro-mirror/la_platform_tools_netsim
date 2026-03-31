// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

/// Hostapd Interface for Network Simulation
use crate::wifi::error::{WifiError, WifiResult};
use bytes::Bytes;
pub use hostapd_rs::hostapd::Hostapd;
use netsim_common::util::os_utils::get_discovery_directory;
use netsim_proto::config::HostapdOptions as ProtoHostapdOptions;
use tokio::sync::mpsc;

pub async fn hostapd_run(
    _opt: ProtoHostapdOptions,
    tx: mpsc::Sender<Bytes>,
    wifi_args: Option<Vec<String>>,
) -> WifiResult<Hostapd> {
    // Create hostapd.conf under discovery directory
    let config_path =
        get_discovery_directory().join(format!("hostapd_{}.conf", std::process::id()));
    let mut hostapd = Hostapd::new(tx, true, config_path);
    if let Some(wifi_values) = wifi_args {
        let ssid = &wifi_values[0];
        let password = wifi_values.get(1).cloned().unwrap_or_default();
        hostapd
            .set_ssid(ssid, password)
            .await
            .map_err(|e| WifiError::Hostapd(format!("Failed to set SSID: {:?}", e)))?;
    }
    if !hostapd.run().await {
        return Err(WifiError::Hostapd("Hostapd run failed".into()));
    }
    Ok(hostapd)
}
