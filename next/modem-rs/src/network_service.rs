// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/network_service.rs

use netsim_model::RegistrationStatus;

use crate::{
    parser::Command,
    types::{ExecutionResult, HandledCommand},
};

const DUMMY_LAC: &str = "2142";
const DUMMY_CID: &str = "0000B804";
const DUMMY_ACT: u8 = 7; // LTE (EUTRAN)

// Holds all state related to the network.
pub struct NetworkService {
    operator_name: String,
    voice_registration: RegistrationStatus,
    data_registration: RegistrationStatus,
    signal_strength: (u8, u8), // (rssi, ber)
    voice_unsol_mode: u8,
    data_unsol_mode: u8,
    lte_unsol_mode: u8,
    radio_power: u8,
}

impl Default for NetworkService {
    fn default() -> Self {
        Self {
            // This will be loaded from config later.
            operator_name: "Android Virtual Operator".to_string(),
            voice_registration: RegistrationStatus::NotRegistered,
            data_registration: RegistrationStatus::NotRegistered,
            signal_strength: (20, 99),
            voice_unsol_mode: 0,
            data_unsol_mode: 0,
            lte_unsol_mode: 0,
            radio_power: 1,
        }
    }
}

impl NetworkService {
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
            self.format_creg_urc(status)
        } else {
            None
        }
    }

    pub fn set_data_registration(&mut self, status: RegistrationStatus) -> Option<String> {
        if self.data_registration != status {
            self.data_registration = status;
            let mut urcs = String::new();
            if let Some(cgreg) = self.format_cgreg_urc(status) {
                urcs.push_str(&cgreg);
            }
            if let Some(cereg) = self.format_cereg_urc(status) {
                urcs.push_str(&cereg);
            }
            if urcs.is_empty() { None } else { Some(urcs) }
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
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    fn format_creg_urc(&self, status: RegistrationStatus) -> Option<String> {
        if self.voice_unsol_mode == 0 {
            None
        } else if self.voice_unsol_mode == 2 {
            Some(format!(
                "+CREG: {},\"{}\",\"{}\",{}\r\n",
                status as u8, DUMMY_LAC, DUMMY_CID, DUMMY_ACT
            ))
        } else {
            Some(format!("+CREG: {}\r\n", status as u8))
        }
    }

    fn format_cgreg_urc(&self, status: RegistrationStatus) -> Option<String> {
        if self.data_unsol_mode == 0 {
            None
        } else if self.data_unsol_mode == 2 {
            Some(format!(
                "+CGREG: {},\"{}\",\"{}\",{}\r\n",
                status as u8, DUMMY_LAC, DUMMY_CID, DUMMY_ACT
            ))
        } else {
            Some(format!("+CGREG: {}\r\n", status as u8))
        }
    }

    fn format_cereg_urc(&self, status: RegistrationStatus) -> Option<String> {
        if self.lte_unsol_mode == 0 {
            None
        } else if self.lte_unsol_mode == 2 {
            Some(format!(
                "+CEREG: {},\"{}\",\"{}\",{}\r\n",
                status as u8, DUMMY_LAC, DUMMY_CID, DUMMY_ACT
            ))
        } else {
            Some(format!("+CEREG: {}\r\n", status as u8))
        }
    }

    pub fn handle_query_signal_strength(&self) -> ExecutionResult {
        let (rssi, ber) = self.signal_strength;
        let response = format!("+CSQ: {},{}\r\n", rssi, ber);
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_query_extended_signal_quality(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_voice_registration(&self) -> ExecutionResult {
        let stat = self.voice_registration as u8;
        let response = if self.voice_unsol_mode == 2 {
            format!("+CREG: 2,{},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, DUMMY_ACT)
        } else {
            format!("+CREG: {},{}\r\n", self.voice_unsol_mode, stat)
        };
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_voice_registration(&mut self, mode: u8) -> ExecutionResult {
        if mode <= 2 {
            self.voice_unsol_mode = mode;
            let mut responses = Vec::new();
            if self.voice_registration != RegistrationStatus::NotRegistered
                && mode > 0
                && let Some(urc) = self.format_creg_urc(self.voice_registration)
            {
                responses.push(urc);
            }
            responses.push("OK\r\n".to_string());
            ExecutionResult::Handled(HandledCommand { responses, action: None })
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_query_data_registration(&self) -> ExecutionResult {
        let stat = self.data_registration as u8;
        let response = if self.data_unsol_mode == 2 {
            format!("+CGREG: 2,{},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, DUMMY_ACT)
        } else {
            format!("+CGREG: {},{}\r\n", self.data_unsol_mode, stat)
        };
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_data_registration(&mut self, mode: u8) -> ExecutionResult {
        if mode <= 2 {
            self.data_unsol_mode = mode;
            let mut responses = Vec::new();
            if self.data_registration != RegistrationStatus::NotRegistered
                && mode > 0
                && let Some(urc) = self.format_cgreg_urc(self.data_registration)
            {
                responses.push(urc);
            }
            responses.push("OK\r\n".to_string());
            ExecutionResult::Handled(HandledCommand { responses, action: None })
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_query_lte_registration(&self) -> ExecutionResult {
        let stat = self.data_registration as u8;
        let response = if self.lte_unsol_mode == 2 {
            format!("+CEREG: 2,{},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, DUMMY_ACT)
        } else {
            format!("+CEREG: {},{}\r\n", self.lte_unsol_mode, stat)
        };
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_lte_registration(&mut self, mode: u8) -> ExecutionResult {
        if mode <= 2 {
            self.lte_unsol_mode = mode;
            let mut responses = Vec::new();
            if self.data_registration != RegistrationStatus::NotRegistered
                && mode > 0
                && let Some(urc) = self.format_cereg_urc(self.data_registration)
            {
                responses.push(urc);
            }
            responses.push("OK\r\n".to_string());
            ExecutionResult::Handled(HandledCommand { responses, action: None })
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_query_radio_power(&self) -> ExecutionResult {
        let response = format!("+CFUN: {}\r\n", self.radio_power);
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_radio_power(
        &mut self,
        power: u8,
        enable_unsolicited_urcs: bool,
    ) -> ExecutionResult {
        if power > 1 {
            return ExecutionResult::Handled(HandledCommand::error());
        }

        let old_power = self.radio_power;
        self.radio_power = power;

        let mut responses = Vec::new();

        if old_power != power {
            if power == 0 {
                self.voice_registration = RegistrationStatus::NotRegistered;
                self.data_registration = RegistrationStatus::NotRegistered;
            } else {
                self.voice_registration = RegistrationStatus::RegisteredHome;
                self.data_registration = RegistrationStatus::RegisteredHome;
            }

            if enable_unsolicited_urcs {
                if let Some(urc) = self.format_creg_urc(self.voice_registration) {
                    responses.push(urc);
                }
                if let Some(urc) = self.format_cgreg_urc(self.data_registration) {
                    responses.push(urc);
                }
                if let Some(urc) = self.format_cereg_urc(self.data_registration) {
                    responses.push(urc);
                }
            }
        }

        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::QueryOperator => self.handle_query_operator(),
            Command::QuerySignalStrength => self.handle_query_signal_strength(),
            Command::QueryExtendedSignalQuality => self.handle_query_extended_signal_quality(),
            Command::QueryVoiceNetworkRegistration => self.handle_query_voice_registration(),
            Command::SetVoiceNetworkRegistration(mode) => self.handle_set_voice_registration(*mode),
            Command::QueryDataNetworkRegistration => self.handle_query_data_registration(),
            Command::SetDataNetworkRegistration(mode) => self.handle_set_data_registration(*mode),
            Command::QueryLteNetworkRegistration => self.handle_query_lte_registration(),
            Command::SetLteNetworkRegistration(mode) => self.handle_set_lte_registration(*mode),
            Command::QueryRadioPower => self.handle_query_radio_power(),
            Command::SetRadioPower(power) => self.handle_set_radio_power(*power, true),
            _ => ExecutionResult::Unhandled,
        }
    }
}
