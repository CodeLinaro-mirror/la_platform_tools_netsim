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
    voice_registration: RegistrationStatus,
    data_registration: RegistrationStatus,
    signal_strength: (u8, u8), // (rssi, ber)
    voice_unsol_mode: u8,
    data_unsol_mode: u8,
    lte_unsol_mode: u8,
    radio_power: u8,
    plmn: String,
    cops_format: u8,
    is_attached: bool,
}

impl Default for NetworkService {
    fn default() -> Self {
        Self {
            voice_registration: RegistrationStatus::NotRegistered,
            data_registration: RegistrationStatus::NotRegistered,
            signal_strength: (20, 99),
            voice_unsol_mode: 0,
            data_unsol_mode: 0,
            lte_unsol_mode: 0,
            radio_power: 1,
            plmn: crate::constants::DEFAULT_PLMN.to_string(),
            cops_format: 0,
            is_attached: false,
        }
    }
}

impl NetworkService {
    pub fn is_attached(&self) -> bool {
        self.is_attached
    }

    pub fn attach_network(&mut self) -> Vec<String> {
        if self.is_attached || self.radio_power == 0 {
            return Vec::new();
        }
        self.is_attached = true;
        self.voice_registration = RegistrationStatus::RegisteredHome;
        self.data_registration = RegistrationStatus::RegisteredHome;

        let mut responses = Vec::new();
        if self.voice_unsol_mode > 0 {
            let stat = self.voice_registration as u8;
            let urc = if self.voice_unsol_mode == 2 {
                format!("+CREG: {},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, DUMMY_ACT)
            } else {
                format!("+CREG: {}\r\n", stat)
            };
            responses.push(urc);
        }
        if self.data_unsol_mode > 0 {
            let stat = self.data_registration as u8;
            let urc = if self.data_unsol_mode == 2 {
                format!("+CGREG: {},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, DUMMY_ACT)
            } else {
                format!("+CGREG: {}\r\n", stat)
            };
            responses.push(urc);
        }
        if self.lte_unsol_mode > 0 {
            let stat = self.data_registration as u8;
            let urc = if self.lte_unsol_mode == 2 {
                format!("+CEREG: {},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, DUMMY_ACT)
            } else {
                format!("+CEREG: {}\r\n", stat)
            };
            responses.push(urc);
        }
        let (rssi, ber) = self.signal_strength;
        responses.push(self.build_csq_response(rssi, ber));

        responses
    }

    fn build_csq_response(&self, rssi: u8, ber: u8) -> String {
        format!("+CSQ: {},{}\r\n", rssi, ber)
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
        let cops_response = match self.cops_format {
            0 => format!("+COPS: 0,0,\"{}\"\r\n", crate::constants::DEFAULT_OPERATOR_NAME_LONG),
            1 => format!("+COPS: 0,1,\"{}\"\r\n", crate::constants::DEFAULT_OPERATOR_NAME_SHORT),
            2 => format!("+COPS: 0,2,{}\r\n", self.plmn),
            _ => "+COPS: 0\r\n".to_string(),
        };
        ExecutionResult::Handled(HandledCommand {
            responses: vec![cops_response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_operator(&mut self, _mode: u8, format: Option<u8>) -> ExecutionResult {
        if let Some(fmt) = format
            && (fmt == 0 || fmt == 1 || fmt == 2)
        {
            self.cops_format = fmt;
        }
        ExecutionResult::Handled(HandledCommand::ok())
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
        let response = self.build_csq_response(rssi, ber);
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

        if old_power != power && power == 0 {
            self.is_attached = false;
            self.voice_registration = RegistrationStatus::NotRegistered;
            self.data_registration = RegistrationStatus::NotRegistered;

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

    pub fn execute(&mut self, command: &Command, enable_unsolicited_urcs: bool) -> ExecutionResult {
        match command {
            Command::QueryOperator => self.handle_query_operator(),
            Command::SetOperator { mode, format, .. } => self.handle_set_operator(*mode, *format),
            Command::QuerySignalStrength => self.handle_query_signal_strength(),
            Command::QueryExtendedSignalQuality => self.handle_query_extended_signal_quality(),
            Command::QueryVoiceNetworkRegistration => self.handle_query_voice_registration(),
            Command::SetVoiceNetworkRegistration(mode) => self.handle_set_voice_registration(*mode),
            Command::QueryDataNetworkRegistration => self.handle_query_data_registration(),
            Command::SetDataNetworkRegistration(mode) => self.handle_set_data_registration(*mode),
            Command::QueryLteNetworkRegistration => self.handle_query_lte_registration(),
            Command::SetLteNetworkRegistration(mode) => self.handle_set_lte_registration(*mode),
            Command::QueryRadioPower => self.handle_query_radio_power(),
            Command::SetRadioPower(power) => {
                self.handle_set_radio_power(*power, enable_unsolicited_urcs)
            }
            _ => ExecutionResult::Unhandled,
        }
    }
}
