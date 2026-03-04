use serde::{Deserialize, Serialize};

use crate::chip::ChipId;

/// Parameters for creating a Cellular chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellCreate {
    // Future Cellular specific properties.
}

/// Cellular technology specific chip information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// A string representing the current state of the cellular modem.
    pub state: String,
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
}
