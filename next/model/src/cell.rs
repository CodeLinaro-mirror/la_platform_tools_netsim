// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::chip::ChipId;

pub const MODEM_STATE_DOWN: &str = "down";
pub const MODEM_STATE_RINGING: &str = "ringing";
pub const MODEM_STATE_IDLE: &str = "idle";

/// Parameters for creating a Cellular chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellCreate {
    pub sim_type: Option<i32>,
    pub sim_profile: Option<String>,
}

/// Quirks for compatibility with different guest-side implementations.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quirks {
    /// Flag for compatibility with Goldfish RIL in SDK 37 and earlier.
    pub goldfish_ril_37_or_earlier: bool,
}

/// Cellular technology specific chip information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
    /// A string representing the current state of the cellular modem.
    pub state: String,
    /// SIM card type (0 = No SIM, 1 = Normal SIM, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sim_type: Option<i32>,
    /// XML SIM ICC profile content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sim_profile: Option<String>,
    /// Quirks for compatibility with different guest-side implementations.
    #[serde(default)]
    pub quirks: Quirks,
    #[serde(default)]
    pub sms_count: u32,
    #[serde(default)]
    pub rssi: u32,
    #[serde(default)]
    pub ber: u32,
    #[serde(default)]
    pub voice_registration: RegistrationStatus,
    #[serde(default)]
    pub data_registration: RegistrationStatus,
    #[serde(default)]
    pub active_calls: Vec<Call>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            radio: crate::chip::Radio::default(),
            state: "idle".to_string(),
            sim_type: None,
            sim_profile: None,
            quirks: Quirks::default(),
            sms_count: 0,
            rssi: 0,
            ber: 0,
            voice_registration: RegistrationStatus::default(),
            data_registration: RegistrationStatus::default(),
            active_calls: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Call {
    pub number: String,
    pub state: CallState,
    pub direction: CallDirection,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum CallState {
    #[default]
    Unknown = 0,
    Active = 1,
    Holding = 2,
    Dialing = 3,
    Alerting = 4,
    Incoming = 5,
    Waiting = 6,
}

/// Standard 3GPP terms for Outgoing (Mobile Originated) and Incoming (Mobile
/// Terminated) calls
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum CallDirection {
    #[default]
    Unknown = 0,
    MobileOriginated = 1,
    MobileTerminated = 2,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum RegistrationStatus {
    #[default]
    NotRegistered = 0,
    RegisteredHome = 1,
    Searching = 2,
    Denied = 3,
    Unknown = 4,
    Roaming = 5,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum RadioTechnology {
    Unknown = 0,
    Gsm = 1,
    Lte = 2,
    Nr = 3,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ModemAction {
    ProcessAtCommand { id: ChipId, command: Vec<u8> },
    SetSignalStrength { id: ChipId, rssi: u8, ber: u8 },
    SetVoiceRegistration { id: ChipId, status: RegistrationStatus },
    SetDataRegistration { id: ChipId, status: RegistrationStatus },
    IncomingCall { target_id: ChipId, number: String },
    RemoteAnswer { id: ChipId },
    RemoteHold { id: ChipId, on_hold: bool },
    RemoteHangup { id: ChipId },
    IncomingSms { id: ChipId, sender: String, text: String },
    IncomingPdu { id: ChipId, pdu: String },
    UpdatePhysicalChannelConfigs { id: ChipId },
    UpdateNetworkTime { id: ChipId, time: String },
    SetSimStatus { id: ChipId, present: bool },
    SetNetworkTechnology { id: ChipId, tech: RadioTechnology },
    SetOperator { id: ChipId, operator: String },
}

/// Cellular specific chip updates.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CellUpdate {
    #[serde(flatten)]
    pub radio: crate::chip::RadioUpdate,
    pub state: Option<String>,
}

impl CellUpdate {
    pub fn apply(&self, cell: &mut Cell) {
        self.radio.apply(&mut cell.radio);
        if let Some(state) = &self.state {
            cell.state = state.clone();
        }
    }
}
