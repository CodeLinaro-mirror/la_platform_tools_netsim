// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/network_service.rs

use netsim_model::{Quirks, RegistrationStatus};
use tracing::{info, warn};

use crate::{
    parser::Command,
    types::{CmeError, ExecutionResult, Response, SignalStrength},
};

const DUMMY_LAC: &str = "2142";
const DUMMY_CID: &str = "0000B804";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationType {
    Voice,
    Data,
    Lte,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkUrc {
    Registration {
        reg_type: RegistrationType,
        status: RegistrationStatus,
        lac: Option<String>,
        cid: Option<String>,
        act: Option<u8>,
    },
    SignalStrength {
        rssi: u8,
        ber: u8,
        act: u8,
    },
}

impl std::fmt::Display for NetworkUrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkUrc::Registration { reg_type, status, lac, cid, act } => {
                let prefix = match reg_type {
                    RegistrationType::Voice => "+CREG",
                    RegistrationType::Data => "+CGREG",
                    RegistrationType::Lte => "+CEREG",
                };
                let stat = *status as u8;
                if let (Some(lac), Some(cid), Some(act)) = (lac, cid, act) {
                    write!(f, "{prefix}: {stat},\"{lac}\",\"{cid}\",{act}\r\n")
                } else {
                    write!(f, "{prefix}: {stat}\r\n")
                }
            }
            NetworkUrc::SignalStrength { rssi, ber, act } => {
                write!(f, "{}", build_csq_response_string(*rssi, *ber, *act))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkResponse {
    OperatorQuery {
        is_registered: bool,
        mode: u8,
        format: u8,
        plmn: String,
    },
    SignalStrength {
        rssi: u8,
        ber: u8,
        act: u8,
    },
    RegistrationQuery {
        reg_type: RegistrationType,
        unsol_mode: u8,
        status: RegistrationStatus,
        lac: Option<String>,
        cid: Option<String>,
        act: Option<u8>,
        quirks: Quirks,
    },
    Ctec {
        current: u8,
        preferred: u32,
    },
    CtecSupported(Vec<u8>),
    CtecSetResult {
        urcs: Vec<NetworkUrc>,
    },
    RadioPower(u8),
    Urcs(Vec<NetworkUrc>),
}

impl std::fmt::Display for NetworkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkResponse::OperatorQuery { is_registered, mode, format, plmn } => {
                if *is_registered && *mode != 2 {
                    match format {
                        0 => write!(
                            f,
                            "+COPS: {mode},0,\"{}\"\r\n",
                            crate::constants::DEFAULT_OPERATOR_NAME_LONG
                        ),
                        1 => write!(
                            f,
                            "+COPS: {mode},1,\"{}\"\r\n",
                            crate::constants::DEFAULT_OPERATOR_NAME_SHORT
                        ),
                        2 => write!(f, "+COPS: {mode},2,{plmn}\r\n"),
                        _ => write!(f, "+COPS: {mode}\r\n"),
                    }
                } else {
                    write!(f, "+COPS: {mode},{format},0\r\n")
                }
            }
            NetworkResponse::SignalStrength { rssi, ber, act } => {
                write!(f, "{}", build_csq_response_string(*rssi, *ber, *act))
            }
            NetworkResponse::RegistrationQuery {
                reg_type,
                unsol_mode,
                status,
                lac,
                cid,
                act,
                quirks,
            } => {
                let prefix = match reg_type {
                    RegistrationType::Voice => "+CREG",
                    RegistrationType::Data => "+CGREG",
                    RegistrationType::Lte => "+CEREG",
                };
                let stat = *status as u8;
                let force_location_info = quirks.goldfish_ril_37_or_earlier;
                if *unsol_mode == 2 || force_location_info {
                    let lac_str = lac.as_deref().unwrap_or(DUMMY_LAC);
                    let cid_str = cid.as_deref().unwrap_or(DUMMY_CID);
                    let act_val = act.unwrap_or(crate::constants::access_technology::LTE);
                    write!(
                        f,
                        "{prefix}: {unsol_mode},{stat},\"{lac_str}\",\"{cid_str}\",{act_val}\r\n"
                    )
                } else {
                    write!(f, "{prefix}: {unsol_mode},{stat}\r\n")
                }
            }
            NetworkResponse::Ctec { current, preferred } => {
                write!(f, "+CTEC: {current},{preferred:X}\r\n")
            }
            NetworkResponse::CtecSupported(techs) => {
                let tech_strs: Vec<String> = techs.iter().map(|t| t.to_string()).collect();
                write!(f, "+CTEC: {}\r\n", tech_strs.join(","))
            }
            NetworkResponse::CtecSetResult { urcs } => {
                write!(f, "+CTEC: DONE\r\n")?;
                for urc in urcs {
                    write!(f, "{urc}")?;
                }
                Ok(())
            }
            NetworkResponse::RadioPower(power) => {
                write!(f, "+CFUN: {power}\r\n")
            }
            NetworkResponse::Urcs(urcs) => {
                for urc in urcs {
                    write!(f, "{urc}")?;
                }
                Ok(())
            }
        }
    }
}

type NetworkResult = Result<Option<NetworkResponse>, ExecutionResult>;

/// Formats the unsolicited signal quality (CSQ) report according to the
/// extended 22-field layout.
///
/// Delegates to the `SignalStrength` struct which encapsulates all signal
/// parameters and correctly invalidates measurements that are not
/// applicable to the currently active radio access technology
/// (`act`).
fn build_csq_response_string(rssi: u8, ber: u8, act: u8) -> String {
    let mut ss = SignalStrength::default();
    match act {
        crate::constants::access_technology::GSM => {
            ss.gsm_rssi = rssi as i32;
            ss.gsm_ber = ber as i32;
        }
        crate::constants::access_technology::WCDMA => {
            ss.wcdma_rssi = rssi as i32;
            ss.wcdma_ber = ber as i32;
        }
        crate::constants::access_technology::LTE => {
            ss.lte_rssi = rssi as i32;
            ss.lte_rsrp = NetworkService::rssi_to_rsrp(rssi);
        }
        crate::constants::access_technology::NR => {
            ss.nr_ss_rsrp = NetworkService::rssi_to_rsrp(rssi);
        }
        _ => {}
    }
    ss.to_csq_response()
}

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
    pub(crate) quirks: Quirks,
}

impl NetworkService {
    pub fn new(quirks: Quirks) -> Self {
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
            // Non-standard default: While 3GPP TS 27.007 § 7.3 specifies format 0 (long format
            // alphanumeric) as the default when AT+COPS is queried without setting
            // format, Goldfish RIL (`reference-ril.c`) in `requestOperator`
            // (`RIL_REQUEST_OPERATOR`) expects `AT+COPS?` to return `response[2]` (`<oper>`)
            // as a 6-digit numeric MCC/MNC string (`310260`). If an alphanumeric operator string is
            // returned, Goldfish RIL logs `requestOperator expected mccmnc to be 6
            // decimal digits` and returns an error. Therefore, format 2 (numeric
            // MCC/MNC) must be the default for Goldfish compatibility.
            cops_format: 2,
            current_network_mode: crate::constants::CTEC_DEFAULT_CURRENT_TECH,
            preferred_network_mode: crate::constants::CTEC_DEFAULT_PREFERRED_MASK,
            is_attached: false,
            act: crate::constants::access_technology::LTE,
            quirks,
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
        self.attach_network_urcs().iter().map(|u| u.to_string()).collect()
    }

    fn attach_network_urcs(&mut self) -> Vec<NetworkUrc> {
        info!(
            "attach_network called! is_attached: {}, radio_power: {}",
            self.is_attached, self.radio_power
        );
        if self.is_attached || self.radio_power == 0 || self.radio_power == 4 {
            return Vec::new();
        }
        self.is_attached = true;
        self.voice_registration = RegistrationStatus::RegisteredHome;
        self.data_registration = RegistrationStatus::RegisteredHome;

        let mut urcs = Vec::new();
        if let Some(urc) = self.format_creg_urc(self.voice_registration) {
            urcs.push(urc);
        }
        if let Some(urc) = self.format_cgreg_urc(self.data_registration) {
            urcs.push(urc);
        }
        if let Some(urc) = self.format_cereg_urc(self.data_registration) {
            urcs.push(urc);
        }
        let (rssi, ber) = self.signal_strength;
        urcs.push(NetworkUrc::SignalStrength { rssi, ber, act: self.act });

        urcs
    }

    fn rssi_to_rsrp(rssi: u8) -> i32 {
        if rssi == crate::constants::CSQ_SIGNAL_UNKNOWN {
            return i32::MAX;
        }
        // Convert CSQ RSSI (0-31) to dBm
        // 0 -> -113 dBm, 31 -> -51 dBm, step 2
        let rssi_dbm = -113 + (rssi as i32 * 2);

        // Estimate RSRP = RSSI - 15 dBm
        let rsrp_dbm = rssi_dbm - 15;

        // AIDL expects -1 * rsrp_dbm
        let rsrp_csq = -rsrp_dbm;

        // Clamp to valid range [44, 140]
        rsrp_csq.clamp(44, 140)
    }

    pub fn set_voice_registration(&mut self, status: RegistrationStatus) -> Option<String> {
        if self.voice_registration != status {
            self.voice_registration = status;
            self.format_creg_urc(status).map(|u| u.to_string())
        } else {
            None
        }
    }

    pub fn set_data_registration(&mut self, status: RegistrationStatus) -> Option<String> {
        if self.data_registration != status {
            self.data_registration = status;
            let mut urcs = String::new();
            if let Some(cgreg) = self.format_cgreg_urc(status) {
                urcs.push_str(&cgreg.to_string());
            }
            if let Some(cereg) = self.format_cereg_urc(status) {
                urcs.push_str(&cereg.to_string());
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
                    urcs.push_str(&creg.to_string());
                }
                if let Some(cgreg) = self.format_cgreg_urc(self.data_registration) {
                    urcs.push_str(&cgreg.to_string());
                }
                if let Some(cereg) = self.format_cereg_urc(self.data_registration) {
                    urcs.push_str(&cereg.to_string());
                }
                let (rssi, ber) = self.signal_strength;
                urcs.push_str(&build_csq_response_string(rssi, ber, self.act));
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

    fn handle_query_operator(&self) -> NetworkResult {
        let is_registered = matches!(
            self.voice_registration,
            RegistrationStatus::RegisteredHome | RegistrationStatus::Roaming
        ) || matches!(
            self.data_registration,
            RegistrationStatus::RegisteredHome | RegistrationStatus::Roaming
        );

        Ok(Some(NetworkResponse::OperatorQuery {
            is_registered,
            mode: self.cops_mode,
            format: self.cops_format,
            plmn: self.plmn.clone(),
        }))
    }

    fn handle_set_operator(
        &mut self,
        mode: u8,
        format: Option<u8>,
        oper: Option<&[u8]>,
    ) -> NetworkResult {
        info!(
            "handle_set_operator: mode={}, format={:?}, oper={:?}",
            mode,
            format,
            oper.map(|o| String::from_utf8_lossy(o))
        );

        if format.is_some_and(|fmt| fmt > 2) {
            return Err(ExecutionResult::error());
        }

        match mode {
            0 => {
                self.cops_mode = 0;
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                }
                let mut urcs = Vec::new();
                if !self.is_attached && self.radio_power == 1 {
                    urcs.extend(self.attach_network_urcs());
                }
                if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
            }
            1 => {
                if let Some(op_bytes) = oper {
                    let op_str = match std::str::from_utf8(op_bytes) {
                        Ok(s) => s,
                        Err(_) => return Err(ExecutionResult::error()),
                    };

                    self.cops_mode = 1;
                    if let Some(fmt) = format {
                        self.cops_format = fmt;
                    }

                    let is_valid_operator = op_str == crate::constants::DEFAULT_PLMN
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_LONG
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_SHORT;

                    if is_valid_operator {
                        self.plmn = crate::constants::DEFAULT_PLMN.to_string();
                        let mut urcs = Vec::new();
                        if !self.is_attached && self.radio_power == 1 {
                            urcs.extend(self.attach_network_urcs());
                        }
                        if urcs.is_empty() {
                            Ok(None)
                        } else {
                            Ok(Some(NetworkResponse::Urcs(urcs)))
                        }
                    } else {
                        self.cops_mode = 0;
                        self.is_attached = false;
                        self.voice_registration = RegistrationStatus::Denied;
                        self.data_registration = RegistrationStatus::Denied;
                        let mut urcs = Vec::new();
                        if let Some(urc) = self.format_creg_urc(self.voice_registration) {
                            urcs.push(urc);
                        }
                        if let Some(urc) = self.format_cgreg_urc(self.data_registration) {
                            urcs.push(urc);
                        }
                        if let Some(urc) = self.format_cereg_urc(self.data_registration) {
                            urcs.push(urc);
                        }
                        Err(ExecutionResult::Error {
                            cme: Some(CmeError::NoNetworkService),
                            urcs: vec![Response::Network(NetworkResponse::Urcs(urcs))],
                        })
                    }
                } else {
                    Err(ExecutionResult::error())
                }
            }
            2 => {
                self.cops_mode = 2;
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                }
                let mut urcs = Vec::new();
                if self.is_attached {
                    self.is_attached = false;
                    self.voice_registration = RegistrationStatus::NotRegistered;
                    self.data_registration = RegistrationStatus::NotRegistered;
                    if let Some(urc) = self.format_creg_urc(self.voice_registration) {
                        urcs.push(urc);
                    }
                    if let Some(urc) = self.format_cgreg_urc(self.data_registration) {
                        urcs.push(urc);
                    }
                    if let Some(urc) = self.format_cereg_urc(self.data_registration) {
                        urcs.push(urc);
                    }
                }
                if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
            }
            3 => {
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                    Ok(None)
                } else {
                    Err(ExecutionResult::error())
                }
            }
            4 => {
                if let Some(op_bytes) = oper {
                    let op_str = match std::str::from_utf8(op_bytes) {
                        Ok(s) => s,
                        Err(_) => return Err(ExecutionResult::error()),
                    };

                    self.cops_mode = 4;
                    if let Some(fmt) = format {
                        self.cops_format = fmt;
                    }

                    let manual_success = op_str == crate::constants::DEFAULT_PLMN
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_LONG
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_SHORT;
                    let mut urcs = Vec::new();
                    if manual_success {
                        self.plmn = crate::constants::DEFAULT_PLMN.to_string();
                        if !self.is_attached && self.radio_power == 1 {
                            urcs.extend(self.attach_network_urcs());
                        }
                    } else {
                        self.cops_mode = 0;
                        if !self.is_attached && self.radio_power == 1 {
                            urcs.extend(self.attach_network_urcs());
                        }
                    }
                    if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
                } else {
                    Err(ExecutionResult::error())
                }
            }
            _ => Err(ExecutionResult::error()),
        }
    }

    fn handle_query_signal_strength(&self) -> NetworkResult {
        let (rssi, ber) = self.signal_strength;
        Ok(Some(NetworkResponse::SignalStrength { rssi, ber, act: self.act }))
    }

    fn handle_query_extended_signal_quality(&self) -> NetworkResult {
        Ok(None)
    }

    fn handle_query_voice_registration(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::RegistrationQuery {
            reg_type: RegistrationType::Voice,
            unsol_mode: self.voice_unsol_mode,
            status: self.voice_registration,
            lac: Some(DUMMY_LAC.to_string()),
            cid: Some(DUMMY_CID.to_string()),
            act: Some(self.act),
            quirks: self.quirks,
        }))
    }

    fn handle_set_voice_registration(&mut self, mode: u8) -> NetworkResult {
        info!(
            "handle_set_voice_registration: mode={}, current_reg={:?}",
            mode, self.voice_registration
        );
        if mode <= 2 {
            self.voice_unsol_mode = mode;
            let mut urcs = Vec::new();
            if self.voice_registration != RegistrationStatus::NotRegistered
                && mode > 0
                && let Some(urc) = self.format_creg_urc(self.voice_registration)
            {
                urcs.push(urc);
            }
            if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
        } else {
            Err(ExecutionResult::error())
        }
    }

    fn handle_query_data_registration(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::RegistrationQuery {
            reg_type: RegistrationType::Data,
            unsol_mode: self.data_unsol_mode,
            status: self.data_registration,
            lac: Some(DUMMY_LAC.to_string()),
            cid: Some(DUMMY_CID.to_string()),
            act: Some(self.act),
            quirks: self.quirks,
        }))
    }

    fn handle_set_data_registration(&mut self, mode: u8) -> NetworkResult {
        if mode <= 2 {
            self.data_unsol_mode = mode;
            let mut urcs = Vec::new();
            if self.data_registration != RegistrationStatus::NotRegistered
                && mode > 0
                && let Some(urc) = self.format_cgreg_urc(self.data_registration)
            {
                urcs.push(urc);
            }
            if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
        } else {
            Err(ExecutionResult::error())
        }
    }

    fn handle_query_lte_registration(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::RegistrationQuery {
            reg_type: RegistrationType::Lte,
            unsol_mode: self.lte_unsol_mode,
            status: self.data_registration,
            lac: Some(DUMMY_LAC.to_string()),
            cid: Some(DUMMY_CID.to_string()),
            act: Some(self.act),
            quirks: self.quirks,
        }))
    }

    fn handle_set_lte_registration(&mut self, mode: u8) -> NetworkResult {
        if mode <= 2 {
            self.lte_unsol_mode = mode;
            let mut urcs = Vec::new();
            if self.data_registration != RegistrationStatus::NotRegistered
                && mode > 0
                && let Some(urc) = self.format_cereg_urc(self.data_registration)
            {
                urcs.push(urc);
            }
            if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
        } else {
            Err(ExecutionResult::error())
        }
    }

    fn handle_query_current_ctec(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::Ctec {
            current: self.current_network_mode,
            preferred: self.preferred_network_mode,
        }))
    }

    fn handle_query_supported_ctec(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::CtecSupported(crate::constants::SUPPORTED_CTEC_INDEXES.to_vec())))
    }

    fn handle_set_ctec(&mut self, current: u8, preferred: &[u8]) -> NetworkResult {
        let preferred_str = match std::str::from_utf8(preferred) {
            Ok(s) => s.trim(),
            Err(_) => return Err(ExecutionResult::error()),
        };
        let preferred_clean = preferred_str.trim_matches('"').trim();
        // Strip hex prefix "0x" or "0X" if present
        let preferred_clean = preferred_clean
            .strip_prefix("0x")
            .or_else(|| preferred_clean.strip_prefix("0X"))
            .unwrap_or(preferred_clean);

        let preferred_mask = match u32::from_str_radix(preferred_clean, 16) {
            Ok(val) => val,
            Err(_) => return Err(ExecutionResult::error()),
        };

        // Validate allowed technologies mask
        let allowed_mask = crate::constants::SUPPORTED_CTEC_INDEXES
            .iter()
            .fold(0u32, |acc, &idx| acc | (1u32 << idx));

        // Validate current tech is supported (current is a mask, e.g. 32 for LTE)
        let current_u32 = current as u32;
        if current_u32.count_ones() != 1 || (current_u32 & !allowed_mask) != 0 {
            return Err(ExecutionResult::error());
        }

        // Validate preferred mask only contains supported technologies
        if (preferred_mask & !allowed_mask) != 0 {
            return Err(ExecutionResult::error());
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

        let mut urcs = Vec::new();
        if self.is_attached {
            if let Some(urc) = self.format_creg_urc(self.voice_registration) {
                urcs.push(urc);
            }
            if let Some(urc) = self.format_cgreg_urc(self.data_registration) {
                urcs.push(urc);
            }
            if let Some(urc) = self.format_cereg_urc(self.data_registration) {
                urcs.push(urc);
            }
            let (rssi, ber) = self.signal_strength;
            urcs.push(NetworkUrc::SignalStrength { rssi, ber, act: self.act });
        }

        Ok(Some(NetworkResponse::CtecSetResult { urcs }))
    }

    fn handle_query_radio_power(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::RadioPower(self.radio_power)))
    }

    fn handle_set_radio_power(
        &mut self,
        power: u8,
        enable_unsolicited_urcs: bool,
    ) -> NetworkResult {
        if power != 0 && power != 1 && power != 4 {
            return Err(ExecutionResult::error());
        }

        let old_power = self.radio_power;
        self.radio_power = power;

        let mut urcs = Vec::new();

        let was_on = old_power == 1;
        let is_on = self.radio_power == 1;

        if was_on && !is_on {
            self.is_attached = false;
            self.voice_registration = RegistrationStatus::NotRegistered;
            self.data_registration = RegistrationStatus::NotRegistered;

            if enable_unsolicited_urcs {
                if let Some(urc) = self.format_creg_urc(self.voice_registration) {
                    urcs.push(urc);
                }
                if let Some(urc) = self.format_cgreg_urc(self.data_registration) {
                    urcs.push(urc);
                }
                if let Some(urc) = self.format_cereg_urc(self.data_registration) {
                    urcs.push(urc);
                }
            }
        }

        if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
    }

    fn format_creg_urc(&self, status: RegistrationStatus) -> Option<NetworkUrc> {
        let force_location_info = self.quirks.goldfish_ril_37_or_earlier;
        if self.voice_unsol_mode == 0 {
            None
        } else if self.voice_unsol_mode == 2 || force_location_info {
            Some(NetworkUrc::Registration {
                reg_type: RegistrationType::Voice,
                status,
                lac: Some(DUMMY_LAC.to_string()),
                cid: Some(DUMMY_CID.to_string()),
                act: Some(self.act),
            })
        } else {
            Some(NetworkUrc::Registration {
                reg_type: RegistrationType::Voice,
                status,
                lac: None,
                cid: None,
                act: None,
            })
        }
    }

    fn format_cgreg_urc(&self, status: RegistrationStatus) -> Option<NetworkUrc> {
        let force_location_info = self.quirks.goldfish_ril_37_or_earlier;
        if self.data_unsol_mode == 0 {
            None
        } else if self.data_unsol_mode == 2 || force_location_info {
            Some(NetworkUrc::Registration {
                reg_type: RegistrationType::Data,
                status,
                lac: Some(DUMMY_LAC.to_string()),
                cid: Some(DUMMY_CID.to_string()),
                act: Some(self.act),
            })
        } else {
            Some(NetworkUrc::Registration {
                reg_type: RegistrationType::Data,
                status,
                lac: None,
                cid: None,
                act: None,
            })
        }
    }

    fn format_cereg_urc(&self, status: RegistrationStatus) -> Option<NetworkUrc> {
        let force_location_info = self.quirks.goldfish_ril_37_or_earlier;
        if self.lte_unsol_mode == 0 {
            None
        } else if self.lte_unsol_mode == 2 || force_location_info {
            Some(NetworkUrc::Registration {
                reg_type: RegistrationType::Lte,
                status,
                lac: Some(DUMMY_LAC.to_string()),
                cid: Some(DUMMY_CID.to_string()),
                act: Some(self.act),
            })
        } else {
            Some(NetworkUrc::Registration {
                reg_type: RegistrationType::Lte,
                status,
                lac: None,
                cid: None,
                act: None,
            })
        }
    }

    pub fn execute(&mut self, command: &Command, enable_unsolicited_urcs: bool) -> ExecutionResult {
        let res = match command {
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
            _ => return ExecutionResult::Unhandled,
        };
        res.into()
    }
}
