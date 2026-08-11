// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::{RadioTechnology, RegistrationStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellAction {
    IncomingCall { number: String },
    UpdateCall,
    EndCall,
    ReceiveSms { sender: String, text: String },
    ReceivePdu { pdu: String },
    SetSignalStrength { rssi: u32, ber: u32 },
    SetVoiceRegistration { status: RegistrationStatus },
    SetDataRegistration { status: RegistrationStatus },
    RemoteAnswer,
    RemoteHold { on_hold: bool },
    SetSimStatus { present: bool },
    SetNetworkTechnology { tech: RadioTechnology },
    SetOperator { operator: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellActionResult {
    Success,
}
