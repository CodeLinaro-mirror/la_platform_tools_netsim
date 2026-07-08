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
    // Future Cellular specific properties.
}

/// Cellular technology specific chip information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    #[serde(flatten)]
    pub radio: crate::chip::Radio,
    /// A string representing the current state of the cellular modem.
    pub state: String,
}

impl Default for Cell {
    fn default() -> Self {
        Self { radio: crate::chip::Radio::default(), state: "idle".to_string() }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum RegistrationStatus {
    NotRegistered = 0,
    RegisteredHome = 1,
    Searching = 2,
    Denied = 3,
    Unknown = 4,
    Roaming = 5,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
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
