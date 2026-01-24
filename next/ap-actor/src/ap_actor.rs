// Copyright 2025-2026 The Android Open Source Project

use crate::ieee802_11::Ieee80211Manager;
use crate::shared;
use crate::wpa_auth;
use netsim_packets::ethernet::MacAddr;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// ID for an Access Point instance within this actor.
pub type ApId = u32;

/// Shared Stream ID for the singleton packet stream (matching SLIRP_ID convention)
pub const WIFI_STREAM_ID: u32 = u32::MAX - 1;

/// Configuration for creating a new Access Point.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApConfig {
    pub ssid: String,
    pub bssid: MacAddr,
    pub channel: u8,
    pub hw_mode: String, // "g", "a", "ad", "ax"
    pub wpa_passphrase: Option<String>,
    #[serde(default = "default_beacon_interval")]
    pub beacon_interval: u16,
}

fn default_beacon_interval() -> u16 {
    100
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApUpdate {
    pub ssid: Option<String>,
}

pub enum ApReq {
    Register {
        stream: tokio::sync::mpsc::UnboundedReceiver<bytes::Bytes>,
        sink: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
    },
}

impl std::fmt::Debug for ApReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApReq::Register { .. } => write!(f, "ApReq::Register {{ ... }}"),
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
    pub(crate) next_ap_id: ApId,
    pub(crate) manager: Ieee80211Manager,
    pub shared_keys: std::sync::Arc<shared::SharedKeyStore>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ApState {
    pub config: ApConfig,
    pub wpa: Option<wpa_auth::WpaAuthenticator>,
}

impl ApActor {
    pub fn new(shared_keys: Option<std::sync::Arc<shared::SharedKeyStore>>) -> Self {
        Self {
            sink: None,
            aps: HashMap::new(),
            next_ap_id: 1,
            manager: Ieee80211Manager::new(),
            shared_keys: shared_keys
                .unwrap_or_else(|| std::sync::Arc::new(shared::SharedKeyStore::new())),
        }
    }
}
