// Copyright 2025-2026 The Android Open Source Project

use std::collections::HashMap;

use netsim_model::{
    ap::{DEFAULT_WIFI_BSSID, DEFAULT_WIFI_SSID},
    chip::{ApCreate, ApUpdate as ModelApUpdate, WifiMode},
    device::Position,
};
use netsim_packets::ethernet::MacAddr;
use serde::{Deserialize, Serialize};

use crate::{ieee802_11::Ieee80211Manager, shared, wpa_auth};

/// ID for an Access Point instance within this actor.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApId(pub u32);

impl std::fmt::Display for ApId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for ApId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<ApId> for u32 {
    fn from(id: ApId) -> Self {
        id.0
    }
}

/// Shared Stream ID for the singleton packet stream (matching SLIRP_ID
/// convention)
pub const WIFI_STREAM_ID: u32 = u32::MAX - 1;

/// Configuration for creating a new Access Point.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApConfig {
    pub ssid: String,
    pub bssid: MacAddr,
    pub channel: u8,
    pub hw_mode: WifiMode,
    pub wpa_passphrase: Option<String>,
    #[serde(default = "default_beacon_interval")]
    pub beacon_interval: u16,
    pub country_code: Option<String>,
    #[serde(default = "default_dtim_period")]
    pub dtim_period: u8,
    #[serde(default)]
    pub hidden_ssid: bool,
    #[serde(default)]
    pub sae: bool,
    #[serde(default = "default_wmm_enabled")]
    pub wmm_enabled: bool,
    #[serde(default)]
    pub enterprise_enabled: bool,
    #[serde(default)]
    pub mac_acl_mode: u8, // 0=Disable, 1=Deny, 2=Allow
    #[serde(default)]
    pub mac_acl_list: Vec<MacAddr>,
    #[serde(default = "default_ftm_responder_enabled")]
    pub ftm_responder_enabled: bool,
    #[serde(default)]
    pub position: Position,
}

fn default_ftm_responder_enabled() -> bool {
    true
}

fn default_wmm_enabled() -> bool {
    true
}

fn default_beacon_interval() -> u16 {
    200
}

fn default_dtim_period() -> u8 {
    2
}

impl Default for ApConfig {
    fn default() -> Self {
        Self {
            ssid: DEFAULT_WIFI_SSID.to_string(),
            bssid: MacAddr::from([0; 6]),
            channel: 6,
            hw_mode: WifiMode::G,
            wpa_passphrase: None,
            beacon_interval: default_beacon_interval(),
            country_code: Some("US".to_string()),
            dtim_period: default_dtim_period(),
            hidden_ssid: false,
            sae: false,
            wmm_enabled: default_wmm_enabled(),
            enterprise_enabled: false,
            mac_acl_mode: 0,
            mac_acl_list: Vec::new(),
            ftm_responder_enabled: default_ftm_responder_enabled(),
            position: Default::default(),
        }
    }
}

impl From<ApConfig> for ApCreate {
    fn from(val: ApConfig) -> Self {
        Self {
            ssid: val.ssid,
            bssid: val.bssid.to_string(),
            channel: val.channel,
            hw_mode: val.hw_mode,
            wpa_passphrase: val.wpa_passphrase,
            beacon_interval: val.beacon_interval,
            country_code: val.country_code,
            dtim_period: val.dtim_period,
            hidden_ssid: val.hidden_ssid,
            sae: val.sae,
            wmm_enabled: val.wmm_enabled,
            enterprise_enabled: val.enterprise_enabled,
            mac_acl_mode: val.mac_acl_mode,
            mac_acl_list: val.mac_acl_list.into_iter().map(|s| s.to_string()).collect(),
            ftm_responder_enabled: val.ftm_responder_enabled,
        }
    }
}

impl TryFrom<ApCreate> for ApConfig {
    type Error = String;

    fn try_from(val: ApCreate) -> Result<Self, Self::Error> {
        Ok(Self {
            ssid: val.ssid,
            bssid: val.bssid.parse().map_err(|e| format!("Invalid BSSID: {}", e))?,
            channel: val.channel,
            hw_mode: val.hw_mode,
            wpa_passphrase: val.wpa_passphrase,
            beacon_interval: val.beacon_interval,
            country_code: val.country_code,
            dtim_period: val.dtim_period,
            hidden_ssid: val.hidden_ssid,
            sae: val.sae,
            wmm_enabled: val.wmm_enabled,
            enterprise_enabled: val.enterprise_enabled,
            mac_acl_mode: val.mac_acl_mode,
            mac_acl_list: val
                .mac_acl_list
                .into_iter()
                .map(|s| s.parse().map_err(|e| format!("Invalid MAC in ACL: {}", e)))
                .collect::<Result<Vec<_>, _>>()?,
            ftm_responder_enabled: val.ftm_responder_enabled,
            position: Default::default(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApUpdate {
    pub ssid: Option<String>,
    pub channel: Option<u8>,
    #[serde(default)]
    pub force_disconnect: Vec<String>,
}

impl From<ModelApUpdate> for ApUpdate {
    fn from(val: ModelApUpdate) -> Self {
        Self { ssid: val.ssid, channel: val.channel, force_disconnect: val.force_disconnect }
    }
}

/// Consolidated update struct for ApActor.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ApActorUpdate {
    pub position: Option<Position>,
    pub enabled: Option<bool>,
    pub ap_update: Option<ApUpdate>,
}

pub enum ApReq {
    Register {
        stream: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Send>>,
        sink: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
        shared_keys: std::sync::Arc<shared::SharedKeyStore>,
        beacon_interval: std::time::Duration,
    },
    Disconnect {
        mac: netsim_packets::ethernet::MacAddr,
    },
}

impl std::fmt::Debug for ApReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApReq::Register { .. } => write!(f, "ApReq::Register {{ ... }}"),
            ApReq::Disconnect { mac } => {
                f.debug_struct("ApReq::Disconnect").field("mac", mac).finish()
            }
        }
    }
}

/// Responses from the ApActor (Output of Action).
#[derive(Debug, Clone)]
pub enum ApResponse {
    Ok,
}

// Internal events removed as we return packets directly now.

#[derive(Clone, Debug)]
pub struct ApActor {
    pub(crate) sink: Option<tokio::sync::mpsc::UnboundedSender<bytes::Bytes>>,
    pub(crate) aps: HashMap<ApId, ApState>,
    pub(crate) manager: Ieee80211Manager,
    pub shared_keys: std::sync::Arc<shared::SharedKeyStore>,
    pub next_ap_id: u32,
    pub beacon_interval: Option<u16>, // In TUs (1024us)
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ApState {
    pub id: ApId,
    pub config: ApConfig,
    pub wpa: Option<wpa_auth::WpaAuthenticator>,
    pub sae_sessions: HashMap<MacAddr, crate::sae::SaeStateMachine>,
    pub eap_sessions: HashMap<MacAddr, crate::eap_auth::EapAuthenticator>,
    pub associations: std::collections::HashSet<MacAddr>,
    pub delayed_frames: std::collections::VecDeque<(std::time::Instant, bytes::Bytes)>,
    pub enabled: bool,
}

impl ApActor {
    pub fn new(shared_keys: std::sync::Arc<shared::SharedKeyStore>) -> Self {
        Self {
            sink: None,
            aps: HashMap::new(),
            manager: Ieee80211Manager::new(),
            shared_keys,
            next_ap_id: 1,
            beacon_interval: None,
        }
    }
}

impl ApState {
    pub fn new(id: ApId, mut config: ApConfig) -> Self {
        if config.bssid.bytes == [0; 6] {
            let mut base_mac = DEFAULT_WIFI_BSSID
                .parse::<MacAddr>()
                .expect("DEFAULT_WIFI_BSSID is a valid MAC address");
            base_mac.bytes[4] = (id.0 >> 8) as u8;
            base_mac.bytes[5] = (id.0 & 0xFF) as u8;
            config.bssid = base_mac;
        }

        Self {
            id,
            config,
            wpa: None,
            sae_sessions: HashMap::new(),
            eap_sessions: HashMap::new(),
            associations: std::collections::HashSet::new(),
            delayed_frames: std::collections::VecDeque::new(),
            enabled: true,
        }
    }
}
