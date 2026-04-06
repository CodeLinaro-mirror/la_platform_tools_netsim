// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/network_service.rs

use netsim_model::RegistrationStatus;

use crate::{
    parser::Command,
    types::{ExecutionResult, HandledCommand},
};

// Holds all state related to the network.
pub struct NetworkService {
    operator_name: String,
    voice_registration: RegistrationStatus,
    data_registration: RegistrationStatus,
    signal_strength: (u8, u8), // (rssi, ber)
}

impl NetworkService {
    /// Creates a new NetworkService.
    pub fn new() -> Self {
        Self {
            // This will be loaded from config later.
            operator_name: "Android Virtual Operator".to_string(),
            voice_registration: RegistrationStatus::NotRegistered,
            data_registration: RegistrationStatus::NotRegistered,
            signal_strength: (20, 99),
        }
    }

    pub fn handle_registration_complete(&mut self) -> ExecutionResult {
        self.voice_registration = RegistrationStatus::RegisteredHome;
        self.data_registration = RegistrationStatus::RegisteredHome;
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["+CREG: 1\r\n".to_string(), "+CGREG: 1\r\n".to_string()],
            action: None,
        })
    }

    pub fn set_voice_registration(&mut self, status: RegistrationStatus) -> Option<String> {
        if self.voice_registration != status {
            self.voice_registration = status;
            Some(format!("+CREG: {}\r\n", status as u8))
        } else {
            None
        }
    }

    pub fn set_data_registration(&mut self, status: RegistrationStatus) -> Option<String> {
        if self.data_registration != status {
            self.data_registration = status;
            Some(format!("+CGREG: {}\r\n", status as u8))
        } else {
            None
        }
    }

    /// Sets the signal strength and bit error rate.
    pub fn set_signal_strength(&mut self, rssi: u8, ber: u8) {
        self.signal_strength = (rssi, ber);
    }

    // --- Pure command handlers ---

    pub fn handle_query_operator(&self) -> ExecutionResult {
        let response = format!("+COPS: 0,0,\"{}\"\r\n", self.operator_name);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_query_signal_strength(&self) -> ExecutionResult {
        let (rssi, ber) = self.signal_strength;
        let response = format!("+CSQ: {},{}\r\n", rssi, ber);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_query_extended_signal_quality(&self) -> ExecutionResult {
        // Not implemented yet, just return OK.
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::QueryOperator => self.handle_query_operator(),
            Command::QuerySignalStrength => self.handle_query_signal_strength(),
            Command::QueryExtendedSignalQuality => self.handle_query_extended_signal_quality(),
            _ => ExecutionResult::Unhandled,
        }
    }
}
