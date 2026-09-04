// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/network_service.rs

use modem_rs_derive::CommandParser;
use netsim_model::{Quirks, RegistrationStatus};
use tracing::{info, warn};

use crate::{
    constants::{DEFAULT_OPERATOR_NAME_LONG, DEFAULT_OPERATOR_NAME_SHORT, DEFAULT_PLMN},
    parser::{QuotedString, parse_raw_data},
    types::{
        AccessTechnology, CmeError, CopsFormat, CopsMode, CtecTechnology, ExecutionResult,
        OperatorStatus, Parsable, Plmn, RadioPowerLevel, RegistrationUnsolicitedMode, Response,
        SignalStrength,
    },
};

/// Network service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum NetworkCommand<'a> {
    #[command(tag = "AT+COPS?")]
    QueryOperator,
    #[command(tag = "AT+COPS=?")]
    QueryAvailableOperators,
    #[command(tag = "AT+COPS=")]
    SetOperator { mode: CopsMode, format: Option<CopsFormat>, oper: Option<QuotedString<'a>> },
    #[command(tag = "AT+CREG?")]
    QueryVoiceNetworkRegistration,
    #[command(tag = "AT+CREG=")]
    SetVoiceNetworkRegistration(RegistrationUnsolicitedMode),
    #[command(tag = "AT+CGREG?")]
    QueryDataNetworkRegistration,
    #[command(tag = "AT+CGREG=")]
    SetDataNetworkRegistration(RegistrationUnsolicitedMode),
    #[command(tag = "AT+CEREG?")]
    QueryLteNetworkRegistration,
    #[command(tag = "AT+CEREG=")]
    SetLteNetworkRegistration(RegistrationUnsolicitedMode),
    #[command(tag = "AT+CFUN?")]
    QueryRadioPower,
    #[command(tag = "AT+CFUN=")]
    SetRadioPower(RadioPowerLevel),
    #[command(tag = "AT+CSQ")]
    QuerySignalStrength,
    #[command(tag = "AT+CESQ")]
    QueryExtendedSignalQuality,
    #[command(tag = "AT+CTEC?")]
    QueryCurrentNetworkTechnology,
    #[command(tag = "AT+CTEC=?")]
    QuerySupportedNetworkTechnology,
    #[command(tag = "AT+CTEC=")]
    SetNetworkTechnology(CtecTechnology, #[parser(parse_raw_data)] &'a [u8]),
}

const DUMMY_LAC: &str = "2142";
const DUMMY_CID: &str = "0000B804";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorInfo {
    pub status: OperatorStatus,
    pub long_name: String,
    pub short_name: String,
    pub numeric: String,
    pub act: AccessTechnology,
}

impl std::fmt::Display for OperatorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({},\"{}\",\"{}\",\"{}\",{})",
            self.status, self.long_name, self.short_name, self.numeric, self.act
        )
    }
}

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
        act: Option<AccessTechnology>,
    },
    SignalStrength {
        rssi: u8,
        ber: u8,
        act: AccessTechnology,
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
        mode: CopsMode,
        format: CopsFormat,
        plmn: String,
        quirks: Quirks,
    },
    SignalStrength {
        rssi: u8,
        ber: u8,
        act: AccessTechnology,
    },
    RegistrationQuery {
        reg_type: RegistrationType,
        unsol_mode: RegistrationUnsolicitedMode,
        status: RegistrationStatus,
        lac: Option<String>,
        cid: Option<String>,
        act: Option<AccessTechnology>,
        quirks: Quirks,
    },
    Ctec {
        current: CtecTechnology,
        preferred: u32,
    },
    CtecSupported(Vec<CtecTechnology>),
    CtecSetResult {
        urcs: Vec<NetworkUrc>,
    },
    RadioPower(RadioPowerLevel),
    Urcs(Vec<NetworkUrc>),
    AvailableOperators(Vec<OperatorInfo>),
}

impl std::fmt::Display for NetworkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkResponse::OperatorQuery { is_registered, mode, format, plmn, quirks } => {
                if *is_registered && *mode != CopsMode::Deregister {
                    match format {
                        CopsFormat::LongAlphanumeric => {
                            write!(f, "+COPS: {mode},0,\"{DEFAULT_OPERATOR_NAME_LONG}\"\r\n")
                        }
                        CopsFormat::ShortAlphanumeric => {
                            write!(f, "+COPS: {mode},1,\"{DEFAULT_OPERATOR_NAME_SHORT}\"\r\n")
                        }
                        CopsFormat::Numeric => write!(f, "+COPS: {mode},2,{plmn}\r\n"),
                    }
                } else if quirks.goldfish_ril_37_or_earlier && *format == CopsFormat::Numeric {
                    // Legacy Goldfish RIL (SDK 37 and earlier) parses numeric format 2 as
                    // operatorNumeric. Returning "+COPS: <mode>,2,0" sets
                    // operatorNumeric to "0" (length 1), causing legacy RIL
                    // to crash with std::out_of_range in operatorNumeric.substr(0, 3). Returning
                    // DEFAULT_PLMN ("310260") ensures operatorNumeric has at
                    // least 3 characters and avoids the RIL crash.
                    write!(f, "+COPS: {mode},2,\"{DEFAULT_PLMN}\"\r\n")
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
                if *unsol_mode == RegistrationUnsolicitedMode::EnableWithLocation
                    || force_location_info
                {
                    let lac_str = lac.as_deref().unwrap_or(DUMMY_LAC);
                    let cid_str = cid.as_deref().unwrap_or(DUMMY_CID);
                    let act_val = act.unwrap_or(AccessTechnology::Lte);
                    write!(
                        f,
                        "{prefix}: {unsol_mode},{stat},\"{lac_str}\",\"{cid_str}\",{act_val}\r\n",
                    )
                } else {
                    write!(f, "{prefix}: {unsol_mode},{stat}\r\n")
                }
            }
            NetworkResponse::Ctec { current, preferred } => {
                write!(f, "+CTEC: {current},{preferred:X}\r\n")
            }
            NetworkResponse::CtecSupported(techs) => {
                let tech_strs: Vec<String> =
                    techs.iter().map(|t| (*t as u8).trailing_zeros().to_string()).collect();
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
            NetworkResponse::AvailableOperators(operators) => {
                write!(f, "+COPS: ")?;
                for (i, op) in operators.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{op}")?;
                }
                write!(f, ",,(")?;
                let modes = [
                    CopsMode::Automatic,
                    CopsMode::Manual,
                    CopsMode::Deregister,
                    CopsMode::SetFormatOnly,
                    CopsMode::ManualAutomatic,
                ];
                for (i, m) in modes.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{m}")?;
                }
                write!(f, "),(")?;
                let formats = [
                    CopsFormat::LongAlphanumeric,
                    CopsFormat::ShortAlphanumeric,
                    CopsFormat::Numeric,
                ];
                for (i, fmt) in formats.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{fmt}")?;
                }
                write!(f, ")\r\n")
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
fn build_csq_response_string(rssi: u8, ber: u8, act: AccessTechnology) -> String {
    let mut ss = SignalStrength::default();
    match act {
        AccessTechnology::Gsm => {
            ss.gsm_rssi = rssi as i32;
            ss.gsm_ber = ber as i32;
        }
        AccessTechnology::Wcdma => {
            ss.wcdma_rssi = rssi as i32;
            ss.wcdma_ber = ber as i32;
        }
        AccessTechnology::Lte => {
            ss.lte_rssi = rssi as i32;
            ss.lte_rsrp = NetworkService::rssi_to_rsrp(rssi);
        }
        AccessTechnology::Nr => {
            ss.nr_ss_rsrp = NetworkService::rssi_to_rsrp(rssi);
        }
    }
    ss.to_csq_response()
}

// Holds all state related to the network.
pub struct NetworkService {
    voice_registration: RegistrationStatus,
    data_registration: RegistrationStatus,
    signal_strength: (u8, u8), // (rssi, ber)
    voice_unsol_mode: RegistrationUnsolicitedMode,
    data_unsol_mode: RegistrationUnsolicitedMode,
    lte_unsol_mode: RegistrationUnsolicitedMode,
    radio_power: RadioPowerLevel,
    plmn: Option<Plmn>,
    cops_mode: CopsMode,
    cops_format: CopsFormat,
    current_network_mode: CtecTechnology,
    preferred_network_mode: u32,
    is_attached: bool,
    act: AccessTechnology,
    pub(crate) quirks: Quirks,
}

impl NetworkService {
    pub fn new(quirks: Quirks, home_plmn: Option<Plmn>) -> Self {
        Self {
            voice_registration: RegistrationStatus::NotRegistered,
            data_registration: RegistrationStatus::NotRegistered,
            signal_strength: (20, 99),
            voice_unsol_mode: RegistrationUnsolicitedMode::default(),
            data_unsol_mode: RegistrationUnsolicitedMode::default(),
            lte_unsol_mode: RegistrationUnsolicitedMode::default(),
            radio_power: RadioPowerLevel::default(),
            plmn: home_plmn,
            cops_mode: CopsMode::Automatic,
            // Non-standard default: While 3GPP TS 27.007 § 7.3 specifies format 0 (long format
            // alphanumeric) as the default when AT+COPS is queried without setting
            // format, Goldfish RIL (`reference-ril.c`) in `requestOperator`
            // (`RIL_REQUEST_OPERATOR`) expects `AT+COPS?` to return `response[2]` (`<oper>`)
            // as a 6-digit numeric MCC/MNC string (`310260` or `311740`). If an alphanumeric
            // operator string is returned, Goldfish RIL logs `requestOperator expected
            // mccmnc to be 6 decimal digits` and returns an error. Therefore, format 2
            // (numeric MCC/MNC) must be the default for Goldfish compatibility.
            cops_format: CopsFormat::Numeric,
            current_network_mode: CtecTechnology::Lte,
            preferred_network_mode: CtecTechnology::Nr as u32,
            is_attached: false,
            act: AccessTechnology::Lte,
            quirks,
        }
    }
}

impl NetworkService {
    pub fn is_attached(&self) -> bool {
        self.is_attached
    }
    pub fn voice_registration(&self) -> RegistrationStatus {
        self.voice_registration
    }
    pub fn data_registration(&self) -> RegistrationStatus {
        self.data_registration
    }
    pub fn signal_strength(&self) -> (u8, u8) {
        self.signal_strength
    }

    pub fn detach_network(&mut self) {
        self.is_attached = false;
    }

    pub fn set_home_plmn(&mut self, plmn: Option<Plmn>) {
        self.plmn = plmn;
    }

    pub fn home_plmn(&self) -> Option<&Plmn> {
        self.plmn.as_ref()
    }

    pub fn set_operator_manual(&mut self, mode: CopsMode, oper: Option<&[u8]>) -> NetworkResult {
        self.handle_set_operator(mode, Some(CopsFormat::LongAlphanumeric), oper)
    }

    pub fn attach_network(&mut self) -> Vec<String> {
        self.attach_network_urcs().iter().map(|u| u.to_string()).collect()
    }

    fn attach_network_urcs(&mut self) -> Vec<NetworkUrc> {
        info!(
            "attach_network called! is_attached: {}, radio_power: {}",
            self.is_attached, self.radio_power
        );
        if self.is_attached
            || self.radio_power == RadioPowerLevel::Minimum
            || self.radio_power == RadioPowerLevel::DisableRf
        {
            return Vec::new();
        }
        self.is_attached = true;
        self.voice_registration = RegistrationStatus::RegisteredHome;
        self.data_registration = RegistrationStatus::RegisteredHome;

        let mut urcs = self.all_registration_urcs();
        let (rssi, ber) = self.signal_strength;
        urcs.push(NetworkUrc::SignalStrength { rssi, ber, act: self.act });

        urcs
    }

    fn rssi_to_rsrp(rssi: u8) -> i32 {
        if rssi == crate::constants::CSQ_SIGNAL_UNKNOWN {
            return i32::MAX;
        }
        // Map CSQ (0-31) linearly to LTE RSRP range [-140, -44] dBm (3GPP TS 36.133).
        // CSQ 0 -> -140 dBm, CSQ 31 -> -47 dBm (step of 3 dBm)
        // rsrp_dbm = -140 + (rssi * 3)
        let rsrp_dbm = -140 + (rssi as i32 * 3);

        // AIDL expects -1 * rsrp_dbm
        let rsrp_csq = -rsrp_dbm;

        // Clamp to 3GPP TS 36.133 valid RSRP range [44, 140] (-44 dBm to -140 dBm)
        rsrp_csq.clamp(44, 140)
    }

    pub fn set_voice_registration(&mut self, status: RegistrationStatus) -> Option<String> {
        self.set_registration(RegistrationType::Voice, status)
    }

    pub fn set_data_registration(&mut self, status: RegistrationStatus) -> Option<String> {
        self.set_registration(RegistrationType::Data, status)
    }

    pub fn set_registration(
        &mut self,
        reg_type: RegistrationType,
        status: RegistrationStatus,
    ) -> Option<String> {
        match reg_type {
            RegistrationType::Voice => {
                if self.voice_registration != status {
                    self.voice_registration = status;
                    self.format_registration_urc(RegistrationType::Voice, status)
                        .map(|u| u.to_string())
                } else {
                    None
                }
            }
            RegistrationType::Data | RegistrationType::Lte => {
                if self.data_registration != status {
                    self.data_registration = status;
                    let mut urcs = String::new();
                    if let Some(cgreg) =
                        self.format_registration_urc(RegistrationType::Data, status)
                    {
                        urcs.push_str(&cgreg.to_string());
                    }
                    if let Some(cereg) = self.format_registration_urc(RegistrationType::Lte, status)
                    {
                        urcs.push_str(&cereg.to_string());
                    }
                    if urcs.is_empty() { None } else { Some(urcs) }
                } else {
                    None
                }
            }
        }
    }

    pub fn set_network_technology(
        &mut self,
        tech: netsim_model::RadioTechnology,
    ) -> Option<String> {
        info!(
            "set_network_technology: tech={tech:?}, current_mode={}, act={}, is_attached={}",
            self.current_network_mode, self.act, self.is_attached
        );
        let (new_mode, new_act) = match tech {
            netsim_model::RadioTechnology::Gsm => (CtecTechnology::Gsm, AccessTechnology::Gsm),
            netsim_model::RadioTechnology::Lte => (CtecTechnology::Lte, AccessTechnology::Lte),
            netsim_model::RadioTechnology::Nr => (CtecTechnology::Nr, AccessTechnology::Nr),
            netsim_model::RadioTechnology::Unknown => {
                warn!("Unknown radio technology {:?}, falling back to LTE", tech);
                (CtecTechnology::Lte, AccessTechnology::Lte)
            }
        };

        if self.current_network_mode != new_mode || self.act != new_act {
            self.current_network_mode = new_mode;
            self.act = new_act;
            if self.is_attached {
                let mut urcs = String::new();
                for urc in self.all_registration_urcs() {
                    urcs.push_str(&urc.to_string());
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
            plmn: self.plmn.as_ref().map(|p| p.to_string()).unwrap_or_default(),
            quirks: self.quirks,
        }))
    }

    fn handle_set_operator(
        &mut self,
        mode: CopsMode,
        format: Option<CopsFormat>,
        oper: Option<&[u8]>,
    ) -> NetworkResult {
        info!(
            "handle_set_operator: mode={mode}, format={format:?}, oper={:?}",
            oper.map(|o| String::from_utf8_lossy(o))
        );

        match mode {
            CopsMode::Automatic => {
                self.cops_mode = CopsMode::Automatic;
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                }
                let mut urcs = Vec::new();
                if !self.is_attached && self.radio_power == RadioPowerLevel::Full {
                    urcs.extend(self.attach_network_urcs());
                }
                if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
            }
            CopsMode::Manual => {
                if let Some(op_bytes) = oper {
                    let op_str = match std::str::from_utf8(op_bytes) {
                        Ok(s) => s,
                        Err(_) => return Err(ExecutionResult::error()),
                    };

                    self.cops_mode = CopsMode::Manual;
                    if let Some(fmt) = format {
                        self.cops_format = fmt;
                    }

                    let is_valid_operator =
                        self.plmn.as_ref().is_some_and(|p| op_str == p.as_str())
                            || op_str == crate::constants::DEFAULT_OPERATOR_NAME_LONG
                            || op_str == crate::constants::DEFAULT_OPERATOR_NAME_SHORT;

                    if is_valid_operator {
                        let mut urcs = Vec::new();
                        if !self.is_attached && self.radio_power == RadioPowerLevel::Full {
                            urcs.extend(self.attach_network_urcs());
                        }
                        if urcs.is_empty() {
                            Ok(None)
                        } else {
                            Ok(Some(NetworkResponse::Urcs(urcs)))
                        }
                    } else {
                        self.cops_mode = CopsMode::Automatic;
                        self.is_attached = false;
                        self.voice_registration = RegistrationStatus::Denied;
                        self.data_registration = RegistrationStatus::Denied;
                        let urcs = self.all_registration_urcs();
                        Err(ExecutionResult::Error {
                            cme: Some(CmeError::NoNetworkService),
                            urcs: vec![Response::Network(NetworkResponse::Urcs(urcs))],
                        })
                    }
                } else {
                    Err(ExecutionResult::error())
                }
            }
            CopsMode::Deregister => {
                self.cops_mode = CopsMode::Deregister;
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                }
                let mut urcs = Vec::new();
                if self.is_attached {
                    self.is_attached = false;
                    self.voice_registration = RegistrationStatus::NotRegistered;
                    self.data_registration = RegistrationStatus::NotRegistered;
                    urcs = self.all_registration_urcs();
                }
                if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
            }
            CopsMode::SetFormatOnly => {
                if let Some(fmt) = format {
                    self.cops_format = fmt;
                    Ok(None)
                } else {
                    Err(ExecutionResult::error())
                }
            }
            CopsMode::ManualAutomatic => {
                if let Some(op_bytes) = oper {
                    let op_str = match std::str::from_utf8(op_bytes) {
                        Ok(s) => s,
                        Err(_) => return Err(ExecutionResult::error()),
                    };

                    self.cops_mode = CopsMode::ManualAutomatic;
                    if let Some(fmt) = format {
                        self.cops_format = fmt;
                    }

                    let manual_success = self.plmn.as_ref().is_some_and(|p| op_str == p.as_str())
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_LONG
                        || op_str == crate::constants::DEFAULT_OPERATOR_NAME_SHORT;
                    let mut urcs = Vec::new();
                    if manual_success {
                        if !self.is_attached && self.radio_power == RadioPowerLevel::Full {
                            urcs.extend(self.attach_network_urcs());
                        }
                    } else {
                        self.cops_mode = CopsMode::Automatic;
                        if !self.is_attached && self.radio_power == RadioPowerLevel::Full {
                            urcs.extend(self.attach_network_urcs());
                        }
                    }
                    if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
                } else {
                    Err(ExecutionResult::error())
                }
            }
        }
    }

    fn handle_query_available_operators(&self) -> NetworkResult {
        let status =
            if self.is_attached { OperatorStatus::Current } else { OperatorStatus::Available };
        let default_op = OperatorInfo {
            status,
            long_name: DEFAULT_OPERATOR_NAME_LONG.to_string(),
            short_name: DEFAULT_OPERATOR_NAME_SHORT.to_string(),
            numeric: self.plmn.as_ref().map(|p| p.to_string()).unwrap_or_default(),
            act: self.act,
        };
        Ok(Some(NetworkResponse::AvailableOperators(vec![default_op])))
    }

    fn handle_query_signal_strength(&self) -> NetworkResult {
        let (rssi, ber) = self.signal_strength;
        Ok(Some(NetworkResponse::SignalStrength { rssi, ber, act: self.act }))
    }

    fn handle_query_extended_signal_quality(&self) -> NetworkResult {
        Ok(None)
    }

    fn unsol_mode(&self, reg_type: RegistrationType) -> RegistrationUnsolicitedMode {
        match reg_type {
            RegistrationType::Voice => self.voice_unsol_mode,
            RegistrationType::Data => self.data_unsol_mode,
            RegistrationType::Lte => self.lte_unsol_mode,
        }
    }

    fn unsol_mode_mut(&mut self, reg_type: RegistrationType) -> &mut RegistrationUnsolicitedMode {
        match reg_type {
            RegistrationType::Voice => &mut self.voice_unsol_mode,
            RegistrationType::Data => &mut self.data_unsol_mode,
            RegistrationType::Lte => &mut self.lte_unsol_mode,
        }
    }

    pub fn registration_status(&self, reg_type: RegistrationType) -> RegistrationStatus {
        match reg_type {
            RegistrationType::Voice => self.voice_registration,
            RegistrationType::Data | RegistrationType::Lte => self.data_registration,
        }
    }

    fn handle_query_registration(&self, reg_type: RegistrationType) -> NetworkResult {
        Ok(Some(NetworkResponse::RegistrationQuery {
            reg_type,
            unsol_mode: self.unsol_mode(reg_type),
            status: self.registration_status(reg_type),
            lac: Some(DUMMY_LAC.to_string()),
            cid: Some(DUMMY_CID.to_string()),
            act: Some(self.act),
            quirks: self.quirks,
        }))
    }

    fn handle_set_registration(
        &mut self,
        reg_type: RegistrationType,
        mode: RegistrationUnsolicitedMode,
    ) -> NetworkResult {
        info!(
            "handle_set_registration({reg_type:?}): mode={mode}, current_reg={:?}",
            self.registration_status(reg_type)
        );
        *self.unsol_mode_mut(reg_type) = mode;
        let mut urcs = Vec::new();
        let status = self.registration_status(reg_type);
        if status != RegistrationStatus::NotRegistered
            && mode != RegistrationUnsolicitedMode::Disable
            && let Some(urc) = self.format_registration_urc(reg_type, status)
        {
            urcs.push(urc);
        }
        if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
    }

    fn handle_query_current_ctec(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::Ctec {
            current: self.current_network_mode,
            preferred: self.preferred_network_mode,
        }))
    }

    fn handle_query_supported_ctec(&self) -> NetworkResult {
        Ok(Some(NetworkResponse::CtecSupported(crate::constants::SUPPORTED_CTEC_TECHS.to_vec())))
    }

    fn handle_set_ctec(&mut self, current: CtecTechnology, preferred: &[u8]) -> NetworkResult {
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
        let allowed_mask = crate::constants::SUPPORTED_CTEC_TECHS
            .iter()
            .fold(0u32, |acc, &tech| acc | (tech as u32));

        // Validate preferred mask only contains supported technologies
        if (preferred_mask & !allowed_mask) != 0 {
            return Err(ExecutionResult::error());
        }

        info!("handle_set_ctec: current={current}, preferred_mask={preferred_mask:#X}");
        self.current_network_mode = current;
        self.preferred_network_mode = preferred_mask;
        self.act = match current {
            CtecTechnology::Gsm => AccessTechnology::Gsm,
            CtecTechnology::Wcdma => AccessTechnology::Wcdma,
            CtecTechnology::Lte => AccessTechnology::Lte,
            CtecTechnology::Nr => AccessTechnology::Nr,
        };
        info!("handle_set_ctec: updated self.act to {}", self.act);

        let mut urcs = Vec::new();
        if self.is_attached {
            urcs = self.all_registration_urcs();
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
        power: RadioPowerLevel,
        enable_unsolicited_urcs: bool,
    ) -> NetworkResult {
        let old_power = self.radio_power;
        self.radio_power = power;

        let mut urcs = Vec::new();

        let was_on = old_power == RadioPowerLevel::Full;
        let is_on = self.radio_power == RadioPowerLevel::Full;

        if was_on && !is_on {
            self.is_attached = false;
            self.voice_registration = RegistrationStatus::NotRegistered;
            self.data_registration = RegistrationStatus::NotRegistered;

            if enable_unsolicited_urcs {
                urcs = self.all_registration_urcs();
            }
        }

        if urcs.is_empty() { Ok(None) } else { Ok(Some(NetworkResponse::Urcs(urcs))) }
    }

    fn format_registration_urc(
        &self,
        reg_type: RegistrationType,
        status: RegistrationStatus,
    ) -> Option<NetworkUrc> {
        let force_location_info = self.quirks.goldfish_ril_37_or_earlier;
        let unsol_mode = self.unsol_mode(reg_type);
        if unsol_mode == RegistrationUnsolicitedMode::Disable {
            None
        } else if unsol_mode == RegistrationUnsolicitedMode::EnableWithLocation
            || force_location_info
        {
            Some(NetworkUrc::Registration {
                reg_type,
                status,
                lac: Some(DUMMY_LAC.to_string()),
                cid: Some(DUMMY_CID.to_string()),
                act: Some(self.act),
            })
        } else {
            Some(NetworkUrc::Registration { reg_type, status, lac: None, cid: None, act: None })
        }
    }

    fn all_registration_urcs(&self) -> Vec<NetworkUrc> {
        let mut urcs = Vec::new();
        if let Some(urc) =
            self.format_registration_urc(RegistrationType::Voice, self.voice_registration)
        {
            urcs.push(urc);
        }
        if let Some(urc) =
            self.format_registration_urc(RegistrationType::Data, self.data_registration)
        {
            urcs.push(urc);
        }
        if let Some(urc) =
            self.format_registration_urc(RegistrationType::Lte, self.data_registration)
        {
            urcs.push(urc);
        }
        urcs
    }

    pub fn execute<'a>(
        &mut self,
        command: &NetworkCommand<'a>,
        enable_unsolicited_urcs: bool,
    ) -> ExecutionResult {
        let res = match command {
            NetworkCommand::QueryOperator => self.handle_query_operator(),
            NetworkCommand::QueryAvailableOperators => self.handle_query_available_operators(),
            NetworkCommand::SetOperator { mode, format, oper } => {
                self.handle_set_operator(*mode, *format, oper.as_deref())
            }
            NetworkCommand::QuerySignalStrength => self.handle_query_signal_strength(),
            NetworkCommand::QueryExtendedSignalQuality => {
                self.handle_query_extended_signal_quality()
            }
            NetworkCommand::QueryVoiceNetworkRegistration => {
                self.handle_query_registration(RegistrationType::Voice)
            }
            NetworkCommand::SetVoiceNetworkRegistration(mode) => {
                self.handle_set_registration(RegistrationType::Voice, *mode)
            }
            NetworkCommand::QueryDataNetworkRegistration => {
                self.handle_query_registration(RegistrationType::Data)
            }
            NetworkCommand::SetDataNetworkRegistration(mode) => {
                self.handle_set_registration(RegistrationType::Data, *mode)
            }
            NetworkCommand::QueryLteNetworkRegistration => {
                self.handle_query_registration(RegistrationType::Lte)
            }
            NetworkCommand::SetLteNetworkRegistration(mode) => {
                self.handle_set_registration(RegistrationType::Lte, *mode)
            }
            NetworkCommand::QueryRadioPower => self.handle_query_radio_power(),
            NetworkCommand::SetRadioPower(power) => {
                self.handle_set_radio_power(*power, enable_unsolicited_urcs)
            }
            NetworkCommand::QueryCurrentNetworkTechnology => self.handle_query_current_ctec(),
            NetworkCommand::QuerySupportedNetworkTechnology => self.handle_query_supported_ctec(),
            NetworkCommand::SetNetworkTechnology(current, preferred) => {
                self.handle_set_ctec(*current, preferred.as_ref())
            }
        };
        res.into()
    }
}
