// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/network_service.rs

use netsim_model::RegistrationStatus;
use tracing::{info, warn};

use crate::{
    parser::Command,
    types::{ExecutionResult, HandledCommand, SignalStrength},
};

const DUMMY_LAC: &str = "2142";
const DUMMY_CID: &str = "0000B804";

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
    cops_mode: u8,
    cops_format: u8,
    current_network_mode: u8,
    preferred_network_mode: u32,
    is_attached: bool,
    act: u8,
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
            cops_mode: 0,
            cops_format: 0,
            current_network_mode: crate::constants::CTEC_DEFAULT_CURRENT_TECH,
            preferred_network_mode: crate::constants::CTEC_DEFAULT_PREFERRED_MASK,
            is_attached: false,
            act: crate::constants::access_technology::LTE,
        }
    }
}

impl NetworkService {
    pub fn is_attached(&self) -> bool {
        self.is_attached
    }

    pub fn detach_network(&mut self) {
        self.is_attached = false;
    }

    pub fn attach_network(&mut self) -> Vec<String> {
        info!(
            "attach_network called! is_attached: {}, radio_power: {}",
            self.is_attached, self.radio_power
        );
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
                format!("+CREG: {},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, self.act)
            } else {
                format!("+CREG: {stat}\r\n")
            };
            responses.push(urc);
        }
        if self.data_unsol_mode > 0 {
            let stat = self.data_registration as u8;
            let urc = if self.data_unsol_mode == 2 {
                format!("+CGREG: {},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, self.act)
            } else {
                format!("+CGREG: {stat}\r\n")
            };
            responses.push(urc);
        }
        if self.lte_unsol_mode > 0 {
            let stat = self.data_registration as u8;
            let urc = if self.lte_unsol_mode == 2 {
                format!("+CEREG: {},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, self.act)
            } else {
                format!("+CEREG: {stat}\r\n")
            };
            responses.push(urc);
        }
        let (rssi, ber) = self.signal_strength;
        responses.push(self.build_csq_response(rssi, ber));

        responses
    }

    /// Formats the unsolicited signal quality (CSQ) report according to the
    /// extended 22-field layout.
    ///
    /// Delegates to the `SignalStrength` struct which encapsulates all signal
    /// parameters and correctly invalidates measurements that are not
    /// applicable to the currently active radio access technology
    /// (`self.act`).
    fn build_csq_response(&self, rssi: u8, ber: u8) -> String {
        let mut ss = SignalStrength::default();
        match self.act {
            crate::constants::access_technology::GSM
            | crate::constants::access_technology::WCDMA => {
                ss.gsm_rssi = rssi as i32;
                ss.gsm_ber = ber as i32;
            }
            crate::constants::access_technology::LTE => {
                if rssi != crate::constants::CSQ_SIGNAL_UNKNOWN {
                    ss.lte_rssi = rssi as i32;
                    ss.lte_rsrp = rssi as i32;
                }
            }
            crate::constants::access_technology::NR => {
                if rssi != crate::constants::CSQ_SIGNAL_UNKNOWN {
                    ss.nr_ss_rsrp = rssi as i32;
                }
            }
            _ => {}
        }
        ss.to_csq_response()
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

    pub fn set_network_technology(
        &mut self,
        tech: netsim_model::RadioTechnology,
    ) -> Option<String> {
        info!(
            "set_network_technology: tech={:?}, current_mode={}, act={}, is_attached={}",
            tech, self.current_network_mode, self.act, self.is_attached
        );
        let (new_mode, new_act) = match tech {
            netsim_model::RadioTechnology::Gsm => {
                (crate::constants::modem_tech::GSM, crate::constants::access_technology::GSM)
            }
            netsim_model::RadioTechnology::Lte => {
                (crate::constants::modem_tech::LTE, crate::constants::access_technology::LTE)
            }
            netsim_model::RadioTechnology::Nr => {
                (crate::constants::modem_tech::NR, crate::constants::access_technology::NR)
            }
            netsim_model::RadioTechnology::Unknown => {
                warn!("Unknown radio technology {:?}, falling back to LTE", tech);
                (crate::constants::modem_tech::LTE, crate::constants::access_technology::LTE)
            }
        };

        if self.current_network_mode != new_mode || self.act != new_act {
            self.current_network_mode = new_mode;
            self.act = new_act;
            if self.is_attached {
                let mut urcs = String::new();
                if let Some(creg) = self.format_creg_urc(self.voice_registration) {
                    urcs.push_str(&creg);
                }
                if let Some(cgreg) = self.format_cgreg_urc(self.data_registration) {
                    urcs.push_str(&cgreg);
                }
                if let Some(cereg) = self.format_cereg_urc(self.data_registration) {
                    urcs.push_str(&cereg);
                }
                let (rssi, ber) = self.signal_strength;
                urcs.push_str(&self.build_csq_response(rssi, ber));
                if urcs.is_empty() { None } else { Some(urcs) }
            } else {
                None
            }
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
        let cops_response = if self.is_attached {
            match self.cops_format {
                0 => format!(
                    "+COPS: {},0,\"{}\"\r\n",
                    self.cops_mode,
                    crate::constants::DEFAULT_OPERATOR_NAME_LONG
                ),
                1 => format!(
                    "+COPS: {},1,\"{}\"\r\n",
                    self.cops_mode,
                    crate::constants::DEFAULT_OPERATOR_NAME_SHORT
                ),
                2 => format!("+COPS: {},2,{}\r\n", self.cops_mode, self.plmn),
                _ => format!("+COPS: {}\r\n", self.cops_mode),
            }
        } else {
            format!("+COPS: {}\r\n", self.cops_mode)
        };
        ExecutionResult::Handled(HandledCommand {
            responses: vec![cops_response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_operator(
        &mut self,
        mode: u8,
        format: Option<u8>,
        oper: Option<&[u8]>,
    ) -> ExecutionResult {
        info!(
            "handle_set_operator: mode={}, format={:?}, oper={:?}",
            mode,
            format,
            oper.map(|o| String::from_utf8_lossy(o))
        );

        if format.is_some_and(|fmt| fmt > 2) {
            return ExecutionResult::Handled(HandledCommand::error());
        }

        match mode {
            0 => {
                self.cops_mode = 0;
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                }
                let mut responses = Vec::new();
                if !self.is_attached && self.radio_power == 1 {
                    responses.extend(self.attach_network());
                }
                responses.push("OK\r\n".to_string());
                ExecutionResult::Handled(HandledCommand { responses, action: None })
            }
            1 => {
                if let Some(op_bytes) = oper {
                    let op_str = match std::str::from_utf8(op_bytes) {
                        Ok(s) => s,
                        Err(_) => return ExecutionResult::Handled(HandledCommand::error()),
                    };

                    let prev_cops_mode = self.cops_mode;
                    self.cops_mode = 1;
                    if let Some(fmt) = format {
                        self.cops_format = fmt;
                    }

                    let is_valid_operator = op_str == crate::constants::DEFAULT_PLMN
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_LONG
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_SHORT;

                    if is_valid_operator {
                        self.plmn = crate::constants::DEFAULT_PLMN.to_string();
                        let mut responses = Vec::new();
                        if !self.is_attached && self.radio_power == 1 {
                            responses.extend(self.attach_network());
                        }
                        responses.push("OK\r\n".to_string());
                        ExecutionResult::Handled(HandledCommand { responses, action: None })
                    } else {
                        self.cops_mode = prev_cops_mode;
                        self.is_attached = false;
                        self.voice_registration = RegistrationStatus::Denied;
                        self.data_registration = RegistrationStatus::Denied;
                        let mut responses = Vec::new();
                        if let Some(urc) = self.format_creg_urc(self.voice_registration) {
                            responses.push(urc);
                        }
                        if let Some(urc) = self.format_cgreg_urc(self.data_registration) {
                            responses.push(urc);
                        }
                        if let Some(urc) = self.format_cereg_urc(self.data_registration) {
                            responses.push(urc);
                        }
                        responses.push("ERROR\r\n".to_string());
                        ExecutionResult::Handled(HandledCommand { responses, action: None })
                    }
                } else {
                    ExecutionResult::Handled(HandledCommand::error())
                }
            }
            2 => {
                self.cops_mode = 2;
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                }
                let mut responses = Vec::new();
                if self.is_attached {
                    self.is_attached = false;
                    self.voice_registration = RegistrationStatus::NotRegistered;
                    self.data_registration = RegistrationStatus::NotRegistered;
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
                responses.push("OK\r\n".to_string());
                ExecutionResult::Handled(HandledCommand { responses, action: None })
            }
            3 => {
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                    ExecutionResult::Handled(HandledCommand::ok())
                } else {
                    ExecutionResult::Handled(HandledCommand::error())
                }
            }
            4 => {
                if let Some(op_bytes) = oper {
                    let op_str = match std::str::from_utf8(op_bytes) {
                        Ok(s) => s,
                        Err(_) => return ExecutionResult::Handled(HandledCommand::error()),
                    };

                    self.cops_mode = 4;
                    if let Some(fmt) = format {
                        self.cops_format = fmt;
                    }

                    let manual_success = op_str == crate::constants::DEFAULT_PLMN
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_LONG
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_SHORT;
                    let mut responses = Vec::new();
                    if manual_success {
                        self.plmn = crate::constants::DEFAULT_PLMN.to_string();
                        if !self.is_attached && self.radio_power == 1 {
                            responses.extend(self.attach_network());
                        }
                    } else {
                        self.cops_mode = 0;
                        if !self.is_attached && self.radio_power == 1 {
                            responses.extend(self.attach_network());
                        }
                    }
                    responses.push("OK\r\n".to_string());
                    ExecutionResult::Handled(HandledCommand { responses, action: None })
                } else {
                    ExecutionResult::Handled(HandledCommand::error())
                }
            }
            _ => ExecutionResult::Handled(HandledCommand::error()),
        }
    }

    fn format_creg_urc(&self, status: RegistrationStatus) -> Option<String> {
        if self.voice_unsol_mode == 0 {
            None
        } else if self.voice_unsol_mode == 2 {
            Some(format!(
                "+CREG: {},\"{}\",\"{}\",{}\r\n",
                status as u8, DUMMY_LAC, DUMMY_CID, self.act
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
                status as u8, DUMMY_LAC, DUMMY_CID, self.act
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
                status as u8, DUMMY_LAC, DUMMY_CID, self.act
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
            format!("+CREG: 2,{},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, self.act)
        } else {
            format!("+CREG: {},{}\r\n", self.voice_unsol_mode, stat)
        };
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_voice_registration(&mut self, mode: u8) -> ExecutionResult {
        info!(
            "handle_set_voice_registration: mode={}, current_reg={:?}",
            mode, self.voice_registration
        );
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
            format!("+CGREG: 2,{},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, self.act)
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
            format!("+CEREG: 2,{},\"{}\",\"{}\",{}\r\n", stat, DUMMY_LAC, DUMMY_CID, self.act)
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

    pub fn handle_query_current_ctec(&self) -> ExecutionResult {
        let response =
            format!("+CTEC: {},{:X}\r\n", self.current_network_mode, self.preferred_network_mode);
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_query_supported_ctec(&self) -> ExecutionResult {
        let tech_strs: Vec<String> =
            crate::constants::SUPPORTED_CTEC_INDEXES.iter().map(|t| t.to_string()).collect();
        let response = format!("+CTEC: {}\r\n", tech_strs.join(","));
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_ctec(&mut self, current: u8, preferred: &[u8]) -> ExecutionResult {
        let preferred_str = match std::str::from_utf8(preferred) {
            Ok(s) => s.trim(),
            Err(_) => return ExecutionResult::Handled(HandledCommand::error()),
        };
        let preferred_clean = preferred_str.trim_matches('"').trim();
        // Strip hex prefix "0x" or "0X" if present
        let preferred_clean = preferred_clean
            .strip_prefix("0x")
            .or_else(|| preferred_clean.strip_prefix("0X"))
            .unwrap_or(preferred_clean);

        let preferred_mask = match u32::from_str_radix(preferred_clean, 16) {
            Ok(val) => val,
            Err(_) => return ExecutionResult::Handled(HandledCommand::error()),
        };

        // Validate allowed technologies mask
        let allowed_mask = crate::constants::SUPPORTED_CTEC_INDEXES
            .iter()
            .fold(0u32, |acc, &idx| acc | (1u32 << idx));

        // Validate current tech is supported (current is a mask, e.g. 32 for LTE)
        let current_u32 = current as u32;
        if current_u32.count_ones() != 1 || (current_u32 & !allowed_mask) != 0 {
            return ExecutionResult::Handled(HandledCommand::error());
        }

        // Validate preferred mask only contains supported technologies
        if (preferred_mask & !allowed_mask) != 0 {
            return ExecutionResult::Handled(HandledCommand::error());
        }

        info!("handle_set_ctec: current={}, preferred_mask={:#X}", current, preferred_mask);
        self.current_network_mode = current;
        self.preferred_network_mode = preferred_mask;
        self.act = match current {
            crate::constants::modem_tech::GSM => crate::constants::access_technology::GSM,
            crate::constants::modem_tech::WCDMA => crate::constants::access_technology::WCDMA,
            crate::constants::modem_tech::LTE => crate::constants::access_technology::LTE,
            crate::constants::modem_tech::NR => crate::constants::access_technology::NR,
            _ => {
                warn!("Unknown current network mode {}, falling back to LTE act", current);
                crate::constants::access_technology::LTE
            }
        };
        info!("handle_set_ctec: updated self.act to {}", self.act);

        let mut responses = Vec::new();
        responses.push("+CTEC: DONE\r\n".to_string());
        if self.is_attached {
            if let Some(creg) = self.format_creg_urc(self.voice_registration) {
                responses.push(creg);
            }
            if let Some(cgreg) = self.format_cgreg_urc(self.data_registration) {
                responses.push(cgreg);
            }
            if let Some(cereg) = self.format_cereg_urc(self.data_registration) {
                responses.push(cereg);
            }
            let (rssi, ber) = self.signal_strength;
            responses.push(self.build_csq_response(rssi, ber));
        }
        responses.push("OK\r\n".to_string());

        ExecutionResult::Handled(HandledCommand { responses, action: None })
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
            Command::SetOperator { mode, format, oper } => {
                self.handle_set_operator(*mode, *format, oper.as_deref())
            }
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
            Command::QueryCurrentNetworkTechnology => self.handle_query_current_ctec(),
            Command::QuerySupportedNetworkTechnology => self.handle_query_supported_ctec(),
            Command::SetNetworkTechnology(current, preferred) => {
                self.handle_set_ctec(*current, preferred.as_ref())
            }
            _ => ExecutionResult::Unhandled,
        }
    }
}
