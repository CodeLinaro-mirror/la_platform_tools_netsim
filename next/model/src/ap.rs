//! Access Point (AP) configuration module.
//!
//! This module defines the types and structures required to configure
//! and manage Access Points within the simulation. It includes the
//! `WifiMode` for PHY layer configuration and `ApCreate`/`ApUpdate`
//! for lifecycle management.

use std::fmt;

use serde::{Deserialize, Serialize};

pub const DEFAULT_WIFI_SSID: &str = "AndroidWifi";
pub const DEFAULT_WIFI_BSSID: &str = "02:15:b2:00:00:00";

/// Supported Wi-Fi 802.11 PHY modes.
///
/// This enum maps 1:1 with the expected "hw_mode" configurations
/// used by hostapd and typically exposed via CLI or configuration files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WifiMode {
    /// 802.11a (5 GHz)
    #[serde(rename = "a")]
    A,
    /// 802.11b (2.4 GHz)
    #[serde(rename = "b")]
    B,
    /// 802.11g (2.4 GHz)
    #[serde(rename = "g")]
    G,
    /// 802.11n (Wi-Fi 4)
    #[serde(rename = "n")]
    N,
    /// 802.11ac (Wi-Fi 5)
    #[serde(rename = "ac")]
    Ac,
    /// 802.11ax (Wi-Fi 6)
    #[serde(rename = "ax")]
    Ax,
}

impl Default for WifiMode {
    fn default() -> Self {
        Self::G
    }
}

impl fmt::Display for WifiMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "a"),
            Self::B => write!(f, "b"),
            Self::G => write!(f, "g"),
            Self::N => write!(f, "n"),
            Self::Ac => write!(f, "ac"),
            Self::Ax => write!(f, "ax"),
        }
    }
}

/// Parameters for creating an Access Point chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApCreate {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub hw_mode: WifiMode,
    pub wpa_passphrase: Option<String>,
    pub beacon_interval: u16,
    pub country_code: Option<String>,
    pub dtim_period: u8,
    pub hidden_ssid: bool,
    pub sae: bool,
    pub wmm_enabled: bool,
    pub enterprise_enabled: bool,
    pub mac_acl_mode: u8,
    pub mac_acl_list: Vec<String>,
    pub ftm_responder_enabled: bool,
}

/// Parameters for updating an Access Point chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApUpdate {
    pub ssid: Option<String>,
    pub channel: Option<u8>,
    #[serde(default)]
    pub force_disconnect: Vec<String>,
}

/// Access Point specific chip information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ap {
    pub config: ApCreate,
    // TODO: Add other state fields if needed
    // Use ignore to skip serialization of fields that are not relevant to the model
    #[serde(skip)]
    pub associations: Vec<String>,
}
