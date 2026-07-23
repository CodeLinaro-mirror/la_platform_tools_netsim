// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use modem_rs_derive::CommandParser;
use tracing::{info, warn};

use crate::{
    config::{DedicatedFile, ElementaryFile, FileSystem, SimFile, SimProfile},
    parser::{ApduData, PinString, QuotedString, parse_raw_data},
    profiles::Profile,
    types::{CmeError, DEFAULT_PIN, ExecutionResult, Parsable},
};

/// SIM service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum SimCommand<'a> {
    #[command(tag = "AT+CPIN?")]
    GetSimStatus,
    #[command(tag = "AT+CPINR=")]
    QueryPinRetries(QuotedString<'a>),
    #[command(tag = "AT+CPIN=")]
    EnterPin(PinString<'a>, Option<PinString<'a>>),
    #[command(tag = "AT+CRSM=")]
    SimIo {
        command: u16,
        file_id: u16,
        p1: u8,
        p2: u8,
        p3: u8,
        data: Option<ApduData<'a>>,
        path: Option<ApduData<'a>>,
    },
    #[command(tag = "AT+CIMI")]
    GetImsi,
    #[command(tag = "AT+CICCID")]
    GetIccid,
    #[command(tag = "AT+CCHO=")]
    OpenLogicalChannel(#[parser(parse_raw_data)] &'a [u8]),
    #[command(tag = "AT+CCHC=")]
    CloseLogicalChannel(u8),
    #[command(tag = "AT+CGLA=")]
    TransmitLogicalChannel(u8, u8, #[parser(parse_raw_data)] &'a [u8]),
    #[command(tag = "AT+CPWD=")]
    ChangePassword(QuotedString<'a>, QuotedString<'a>, QuotedString<'a>),
    /// VENDOR: Query PIN retries
    #[command(tag = "AT+SPIC")]
    QueryPinRetriesSpic,
    /// 3GPP2 C.S0023: Set CDMA subscription source
    #[command(tag = "AT+CCSS=")]
    SetCdmaSubscriptionSource(u8),
    /// 3GPP2 C.S0023: Query CDMA subscription source
    #[command(tag = "AT+CCSS?")]
    QueryCdmaSubscriptionSource,
    /// 3GPP2 C.S0023: Set CDMA roaming preference
    #[command(tag = "AT+WRMP=")]
    SetCdmaRoamingPreference(u8),
    /// 3GPP2 C.S0023: Query CDMA roaming preference
    #[command(tag = "AT+WRMP?")]
    QueryCdmaRoamingPreference,
    /// Generic SIM access (+CSIM)
    #[command(tag = "AT+CSIM=")]
    GenericSimAccess(u32, ApduData<'a>),
    #[command(tag = "AT+MBAU=")]
    SimAuthentication(#[parser(parse_raw_data)] &'a [u8]),
    /// SIM authentication (Vendor caret version)
    #[command(tag = "AT^MBAU=")]
    SimAuthenticationVendor(#[parser(parse_raw_data)] &'a [u8]),
    /// VENDOR: Update phone number
    #[command(tag = "AT+REMOTEUPADATEPHONENUMBER")]
    UpdatePhoneNumber(#[parser(parse_raw_data)] &'a [u8]),
}

const MIN_PIN_LEN: usize = 4;
const MAX_PIN_LEN: usize = 8;
const PUK_LEN: usize = 8;
const DEFAULT_PIN_RETRIES: u32 = 3;
const DEFAULT_PUK_RETRIES: u32 = 10;
#[allow(dead_code)]
const CLA_UNSUPPORTED: u8 = 0xFF;

const DEFAULT_PUK: &str = "12345678";

const APDU_SELECT: u16 = 0xA4;
const APDU_READ_BINARY: u16 = 0xB0;
const APDU_READ_RECORD: u16 = 0xB2;
#[allow(dead_code)]
const APDU_GET_RESPONSE: u16 = 0xC0;
const APDU_UPDATE_BINARY: u16 = 0xD6;
const APDU_UPDATE_RECORD: u16 = 0xDC;
const APDU_STATUS: u16 = 0xF2;

const DEFAULT_FALLBACK_IMSI: &str = "310260123456789";
const DEFAULT_FALLBACK_ICCID: &str = "89012608640220133897";
const EF_FPLMN_DATA_FALLBACK: &str = "FFFFFFFFFFFFFFFFFFFFFFFF";
const EF_MSISDN_RECORD_FALLBACK: &str = "000000000000000000000000000007915155214365F7FFFFFFFFFFFF";
const STATUS_FCP_HEX: &str = "62338202782183023F00A50C80016187010183040007DBF08A01058B062F0601020002C60C90016083010183010A83010D8102FFFF";

// SIM Elementary File IDs (EF IDs)
const EF_IMSI_ID: u16 = 0x6F07;
const EF_ICCID_ID: u16 = 0x2FE2;
const EF_FPLMN_ID: u16 = 0x6F7B;
const EF_MSISDN_ID: u16 = 0x6F40;
const EF_MBDN_ID: u16 = 0x6FC7;
const EF_AD_ID: u16 = 0x6FAD;

// APDU Instruction Bytes (INS)
const INS_SELECT: u8 = 0xA4;
const INS_READ_BINARY: u8 = 0xB0;
#[allow(dead_code)]
const INS_GET_RESPONSE: u8 = 0xC0;
#[allow(dead_code)]
const INS_GET_DATA: u8 = 0xCA;
const INS_STATUS: u8 = 0xF2;
const INS_MANAGE_CHANNEL: u8 = 0x70;

const P2_INVALID_SELECT: u8 = 0xF0;

const MANAGE_CHANNEL_ACTION_OPEN: u8 = 0x00;
const MANAGE_CHANNEL_ACTION_CLOSE: u8 = 0x80;

// APDU Status Words (SW)
const SW_SUCCESS: &str = "9000";
const SW_INCORRECT_PARAMS: &str = "6A86";
const SW_FILE_NOT_FOUND: &str = "6A82";
const SW_CLASS_NOT_SUPPORTED: &str = "6E00";
const SW_WRONG_LENGTH: &str = "6700";
const SW_INS_NOT_SUPPORTED: &str = "6D00";
const SW_TECHNICAL_PROBLEM: &str = "6F00";
const SW_NO_CHANNEL_AVAILABLE: &str = "6A81";
const SW_REFERENCED_DATA_NOT_FOUND: &str = "6A88";
#[allow(dead_code)]
const SW_INCORRECT_P1_P2: &str = "6B00";

// Hex Data Templates

// SIM Authentication mock challenges and responses
const AUTH_CHALLENGE_1: &str = "2713AB0BA8E8E7D8F1D74545BA03F563";
const AUTH_RESPONSE_1: &str = "^MBAU: 0,8F2980FC3872FF89,E9620240\r\n";
const AUTH_CHALLENGE_2: &str = "C3718EC16B3C2A66F8A7200A64069F04";
const AUTH_RESPONSE_2: &str = "^MBAU: 0,CFDA6C980502DA48,F7E53577\r\n";
const AUTH_CHALLENGE_3: &str = "11111111111111111111111111111111";
const AUTH_RESPONSE_3: &str = "^MBAU: 0,0000000000000000,00000000\r\n";
const AUTH_CHALLENGE_4: &str = "11111111111111111111111111111111,12351417161900001130131215141716";
const AUTH_RESPONSE_4: &str = "^MBAU: 0,111013121514171619181B1A1D1C1F1E,1013121514171619181B1A1D1C1F1E11,13121514171619181B1A1D1C1F1E1110\r\n";

// Represents the state of the SIM card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimState {
    Absent,
    Ready,
    PinRequired,
    PukRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimResponse {
    PinStatus(RequiredPin),
    RestrictedSimAccess { status_byte_1: u8, status_byte_2: u8, data: Option<String> },
    Imsi(String),
    Iccid(String),
    OpenLogicalChannel(u8),
    CloseLogicalChannel,
    GenericLogicalChannelAccess(String),
    GenericSimAccess(String),
    CdmaSubscriptionSource(u8),
    CdmaRoamingPreference(u8),
    SimAuthentication(String),
    FacilityLockStatus(u8),
    PinRetriesSpic(u32),
    PinRemainingAttempts { pin_type: String, retries: u32, default_retries: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredPin {
    None,
    SimPin,
    SimPuk,
}

impl std::fmt::Display for SimResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimResponse::PinStatus(status) => match status {
                RequiredPin::None => write!(f, "+CPIN: READY\r\n"),
                RequiredPin::SimPin => write!(f, "+CPIN: SIM PIN\r\n"),
                RequiredPin::SimPuk => write!(f, "+CPIN: SIM PUK\r\n"),
            },
            SimResponse::RestrictedSimAccess { status_byte_1, status_byte_2, data } => {
                if let Some(d) = data {
                    write!(f, "+CRSM: {status_byte_1},{status_byte_2},{d}\r\n")
                } else {
                    write!(f, "+CRSM: {status_byte_1},{status_byte_2}\r\n")
                }
            }
            SimResponse::Imsi(imsi) => write!(f, "{imsi}\r\n"),
            SimResponse::Iccid(iccid) => write!(f, "{iccid}\r\n"),
            SimResponse::OpenLogicalChannel(channel_id) => write!(f, "{channel_id}\r\n"),
            SimResponse::CloseLogicalChannel => write!(f, "+CCHC\r\n"),
            SimResponse::GenericLogicalChannelAccess(resp) => write!(f, "{resp}"),
            SimResponse::GenericSimAccess(resp) => write!(f, "+CSIM: {resp}\r\n"),
            SimResponse::CdmaSubscriptionSource(source) => write!(f, "+CCSS: {source}\r\n"),
            SimResponse::CdmaRoamingPreference(pref) => write!(f, "+WRMP: {pref}\r\n"),
            SimResponse::SimAuthentication(resp) => write!(f, "{resp}"),
            SimResponse::FacilityLockStatus(status) => write!(f, "+CLCK: {status}\r\n"),
            SimResponse::PinRetriesSpic(retries) => write!(f, "+SPIC: {retries}\r\n"),
            SimResponse::PinRemainingAttempts { pin_type, retries, default_retries } => {
                write!(f, "+CPINR: \"{pin_type}\",{retries},{default_retries}\r\n")
            }
        }
    }
}

type SimResult = Result<Option<SimResponse>, ExecutionResult>;

// Holds all state related to the SIM card.
pub struct SimService {
    state: SimState,
    pin_enabled: bool,
    imsi: String,
    iccid: String,
    msisdn: String,
    mbdn_records: Vec<String>,
    pin1: String,
    puk1: String,
    pin1_retries: u32,
    puk1_retries: u32,
    pin2_retries: u32,
    puk2_retries: u32,
    fs: FileSystem,
    sms_messages: HashMap<u8, Vec<u8>>,
    logical_channels: [bool; 4],
    selected_aids: [Option<String>; 4],
    selected_files: [Option<u16>; 4],
    cdma_subscription_source: u8,
    cdma_roaming_preference: u8,
    fplmn: String,
    profile: &'static Profile,
}

impl SimService {
    /// Creates a new SimService from a SIM profile configuration.
    pub fn new(profile: &SimProfile, sim_type: Option<i32>) -> Self {
        let pin_enabled =
            matches!(profile.pin_profile.state.as_str(), "EnabledNotVerified" | "EnabledVerified");
        let state = if profile.pin_profile.state.as_str() == "EnabledNotVerified" {
            SimState::PinRequired
        } else {
            SimState::Ready
        };

        let imsi = if profile.imsi.is_empty() {
            DEFAULT_FALLBACK_IMSI.to_string()
        } else {
            profile.imsi.clone()
        };

        let iccid = if profile.iccid.is_empty() {
            DEFAULT_FALLBACK_ICCID.to_string()
        } else {
            profile.iccid.clone()
        };

        let msisdn = profile.msisdn.clone();

        let selected_profile = match sim_type {
            Some(2) => &crate::profiles::PROFILE_CTS,
            _ => &crate::profiles::PROFILE_DEFAULT,
        };

        Self {
            state,
            pin_enabled,
            imsi,
            iccid,
            msisdn,
            mbdn_records: vec!["F".repeat(76); 4],
            pin1: if !profile.pin_profile.pin1.is_empty() {
                profile.pin_profile.pin1.clone()
            } else {
                DEFAULT_PIN.to_string()
            },
            puk1: if !profile.pin_profile.puk1.is_empty() {
                profile.pin_profile.puk1.clone()
            } else {
                DEFAULT_PUK.to_string()
            },
            pin1_retries: DEFAULT_PIN_RETRIES,
            puk1_retries: DEFAULT_PUK_RETRIES,
            pin2_retries: DEFAULT_PIN_RETRIES,
            puk2_retries: DEFAULT_PUK_RETRIES,
            fs: profile.sim_io.file_system.clone(),
            sms_messages: HashMap::new(),
            // Channel 0 is the basic channel and is always open by default.
            logical_channels: [true, false, false, false],
            selected_aids: [None, None, None, None],
            selected_files: [None; 4],
            cdma_subscription_source: 0,
            cdma_roaming_preference: 0,
            fplmn: EF_FPLMN_DATA_FALLBACK.to_string(),
            profile: selected_profile,
        }
    }

    fn lookup_simio(&self, command: u16, file_id: u16, p1: u8, p2: u8, p3: u8) -> Option<String> {
        let file_id_str = format!("{file_id:04X}");
        let cmd_hex = format!("{command:02X}");
        let p1_hex = format!("{p1:X}");
        let p2_hex = format!("{p2:X}");
        let p3_hex = format!("{p3:X}");

        let file_mapping = self.profile.simio_files.iter().find(|fm| fm.file_id == file_id_str)?;
        for m in file_mapping.mappings {
            if m.cmd == cmd_hex && m.p1 == p1_hex && m.p2 == p2_hex && m.p3 == p3_hex {
                return Some(m.response.to_string());
            }
        }
        None
    }

    fn lookup_cgla(&self, aid: &str, file_id: Option<u16>, command: &str) -> Option<String> {
        let adf = self.profile.adfs.iter().find(|am| am.aid == aid)?;
        if let Some(fid) = file_id {
            let fid_str = format!("{fid:04X}");
            if let Some(file_mock) = adf.files.iter().find(|f| f.id == fid_str)
                && let Some(m) = file_mock.cgla.iter().find(|m| m.cmd == command)
            {
                return Some(m.response.to_string());
            }
        }
        adf.cgla.iter().find(|m| m.cmd == command).map(|m| m.response.to_string())
    }

    fn lookup_csim(&self, command: &str) -> Option<String> {
        let adf = self.profile.adfs.iter().find(|am| am.aid == "CSIM")?;
        adf.csim.iter().find(|m| m.cmd == command).map(|m| m.response.to_string())
    }

    pub fn set_msisdn(&mut self, msisdn: &str) {
        self.msisdn = msisdn.to_string();
    }

    pub fn get_cpin_urc(&self) -> Option<String> {
        let status = match self.state {
            SimState::Absent => return Some("+CPIN: ABSENT\r\n".to_string()),
            SimState::Ready => RequiredPin::None,
            SimState::PinRequired => RequiredPin::SimPin,
            SimState::PukRequired => RequiredPin::SimPuk,
        };
        Some(format!("{}", SimResponse::PinStatus(status)))
    }

    pub fn is_present(&self) -> bool {
        self.state != SimState::Absent
    }

    pub fn set_present(&mut self, present: bool) -> bool {
        let old_state = self.state;
        if present {
            if self.state == SimState::Absent {
                self.state = if self.puk1_retries == 0 || self.pin1_retries == 0 {
                    SimState::PukRequired
                } else if self.pin_enabled {
                    SimState::PinRequired
                } else {
                    SimState::Ready
                };
            }
        } else {
            self.state = SimState::Absent;
        }
        self.state != old_state
    }

    // --- Public API for other services ---

    pub fn get_sms_count(&self) -> usize {
        self.sms_messages.len()
    }

    pub fn store_sms(&mut self, pdu: &[u8]) -> Option<u8> {
        if !self.is_present() {
            return None;
        }
        let index = self.sms_messages.len() as u8 + 1;
        self.sms_messages.insert(index, pdu.to_vec());
        Some(index)
    }

    pub fn read_sms(&self, index: u8) -> Result<Option<Vec<u8>>, CmeError> {
        if !self.is_present() {
            return Err(CmeError::SimNotInserted);
        }
        if let Some(pdu) = self.sms_messages.get(&index) { Ok(Some(pdu.clone())) } else { Ok(None) }
    }

    pub fn delete_sms(&mut self, index: u8) -> bool {
        if !self.is_present() {
            return false;
        }
        self.sms_messages.remove(&index).is_some()
    }

    // --- Pure command handlers ---

    fn handle_get_sim_status(&self) -> SimResult {
        let status = match self.state {
            SimState::Absent => unreachable!("Absent state handled in execute"),
            SimState::Ready => RequiredPin::None,
            SimState::PinRequired => RequiredPin::SimPin,
            SimState::PukRequired => RequiredPin::SimPuk,
        };
        Ok(Some(SimResponse::PinStatus(status)))
    }

    fn handle_enter_pin(&mut self, pin_or_puk: PinString, new_pin: Option<PinString>) -> SimResult {
        let pin_or_puk_len = pin_or_puk.as_ref().len();
        match self.state {
            SimState::Absent => unreachable!("Absent state handled in execute"),
            SimState::Ready => {
                if pin_or_puk_len == 0 {
                    Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
                } else {
                    Ok(None)
                }
            }
            SimState::PinRequired => {
                if !(MIN_PIN_LEN..=MAX_PIN_LEN).contains(&pin_or_puk_len) {
                    return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword));
                }
                if pin_or_puk.as_ref() == self.pin1.as_bytes() {
                    self.state = SimState::Ready;
                    self.pin1_retries = DEFAULT_PIN_RETRIES;
                    Ok(None)
                } else {
                    self.pin1_retries = self.pin1_retries.saturating_sub(1);
                    if self.pin1_retries == 0 {
                        self.state = SimState::PukRequired;
                    }
                    Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
                }
            }
            SimState::PukRequired => {
                if pin_or_puk_len != PUK_LEN {
                    return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword));
                }
                if let Some(new_pin) = new_pin {
                    if !(MIN_PIN_LEN..=MAX_PIN_LEN).contains(&new_pin.as_ref().len()) {
                        return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword));
                    }
                    if pin_or_puk.as_ref() == self.puk1.as_bytes() {
                        self.pin1 = String::from_utf8_lossy(new_pin.as_ref()).to_string();
                        self.state = SimState::Ready;
                        self.pin1_retries = DEFAULT_PIN_RETRIES;
                        self.puk1_retries = DEFAULT_PUK_RETRIES;
                        Ok(None)
                    } else {
                        self.puk1_retries = self.puk1_retries.saturating_sub(1);
                        Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
                    }
                } else {
                    Err(ExecutionResult::cme_error(CmeError::IncorrectParameters))
                }
            }
        }
    }

    fn handle_get_imsi(&self) -> SimResult {
        Ok(Some(SimResponse::Imsi(self.imsi.clone())))
    }

    fn handle_get_iccid(&self) -> SimResult {
        Ok(Some(SimResponse::Iccid(self.iccid.clone())))
    }

    fn update_sim_file(
        &mut self,
        command: u16,
        file_id: u16,
        p1: u8,
        p2: u8,
        hex_str: &str,
    ) -> Result<(), (u8, u8)> {
        // Update in memory fields first
        match file_id {
            EF_ICCID_ID => {
                self.iccid = hex_str.to_string();
            }
            EF_IMSI_ID => {
                if let Some(dec_imsi) = decode_imsi(hex_str) {
                    self.imsi = dec_imsi;
                }
            }
            EF_MSISDN_ID => {
                if let Some(dec_msisdn) = decode_msisdn(hex_str) {
                    self.msisdn = dec_msisdn;
                }
            }
            EF_FPLMN_ID => {
                self.fplmn = hex_str.to_string();
            }
            EF_MBDN_ID => {
                let record_idx = (p1 as usize).saturating_sub(1);
                if record_idx < self.mbdn_records.len() {
                    self.mbdn_records[record_idx] = hex_str.to_string();
                }
            }
            _ => {}
        }

        // Update in the file system if it exists there
        if let Some(ef) = find_ef_mut(&mut self.fs.master_file, &format!("{file_id:04X}")) {
            if command == APDU_UPDATE_BINARY {
                let offset = ((p1 as usize) << 8) | (p2 as usize);
                let mut ef_bytes = hex::decode(&ef.data).unwrap_or_default();
                let new_bytes = hex::decode(hex_str).unwrap_or_default();
                if offset + new_bytes.len() > ef_bytes.len() {
                    ef_bytes.resize(offset + new_bytes.len(), 0xFF);
                }
                ef_bytes[offset..offset + new_bytes.len()].copy_from_slice(&new_bytes);
                ef.data = hex::encode_upper(ef_bytes);
                Ok(())
            } else if command == APDU_UPDATE_RECORD {
                // UPDATE RECORD
                if let Some(rec_len) = ef.record_len
                    && rec_len > 0
                {
                    if p1 == 0 {
                        return Err((106, 134)); // 0x6A86 SW_INCORRECT_PARAMS
                    }
                    let rec_len_hex = rec_len * 2;
                    if hex_str.len() != rec_len_hex {
                        return Err((103, 0)); // 0x6700 SW_WRONG_LENGTH
                    }
                    let record_num = p1 as usize;
                    let start = (record_num - 1) * rec_len_hex;
                    let end = start + rec_len_hex;
                    if end > ef.data.len() {
                        return Err((106, 136)); // 0x6A88 SW_REFERENCED_DATA_NOT_FOUND
                    }
                    ef.data.replace_range(start..end, &hex_str.to_ascii_uppercase());
                    return Ok(());
                }
                ef.data = hex_str.to_ascii_uppercase();
                Ok(())
            } else {
                Err((106, 134)) // 0x6A86 SW_INCORRECT_PARAMS
            }
        } else if matches!(
            file_id,
            EF_ICCID_ID | EF_IMSI_ID | EF_MSISDN_ID | EF_FPLMN_ID | EF_MBDN_ID | EF_AD_ID
        ) {
            Ok(())
        } else {
            Err((106, 130)) // 0x6A82 SW_FILE_NOT_FOUND
        }
    }

    fn handle_sim_io(
        &mut self,
        command: u16,
        file_id: u16,
        p1: u8,
        p2: u8,
        p3: u8,
        data: Option<String>,
    ) -> SimResult {
        // 1. Handle UPDATE BINARY and UPDATE RECORD
        if command == APDU_UPDATE_BINARY || command == APDU_UPDATE_RECORD {
            let (sw1, sw2) = if let Some(hex_str) = data {
                match self.update_sim_file(command, file_id, p1, p2, &hex_str) {
                    Ok(()) => (144, 0),
                    Err(err) => err,
                }
            } else {
                (106, 134)
            };
            return Ok(Some(SimResponse::RestrictedSimAccess {
                status_byte_1: sw1,
                status_byte_2: sw2,
                data: None,
            }));
        }

        // 2. Try to read from the loaded FileSystem first (for READ BINARY and SELECT)
        if command == APDU_READ_BINARY {
            if let Some(ef) = find_ef(&self.fs.master_file, &format!("{file_id:04X}")) {
                let offset = ((p1 as usize) << 8) | (p2 as usize);
                let length = p3 as usize;
                let ef_bytes = hex::decode(&ef.data).unwrap_or_default();
                let end = std::cmp::min(offset + length, ef_bytes.len());
                let sliced = if offset < ef_bytes.len() { &ef_bytes[offset..end] } else { &[] };
                return Ok(Some(SimResponse::RestrictedSimAccess {
                    status_byte_1: 144,
                    status_byte_2: 0,
                    data: Some(hex::encode_upper(sliced)),
                }));
            }
        } else if command == APDU_SELECT
            && find_df(&self.fs.master_file, &format!("{file_id:04X}")).is_some()
        {
            return Ok(Some(SimResponse::RestrictedSimAccess {
                status_byte_1: 144,
                status_byte_2: 0,
                data: Some("6210".to_string()),
            }));
        } else if command == APDU_STATUS {
            // Return FCP template for Master File (MF)
            return Ok(Some(SimResponse::RestrictedSimAccess {
                status_byte_1: 144,
                status_byte_2: 0,
                data: Some(STATUS_FCP_HEX.to_string()),
            }));
        }

        // 3. Fallback to default iccprofile sim0 mappings (essential for boot)
        let crsm_result = match (command, file_id) {
            // Dynamic overrides for mutable files
            (APDU_READ_BINARY, EF_ICCID_ID) => Some((144, 0, Some(self.iccid.clone()))),
            (APDU_READ_BINARY, EF_FPLMN_ID) => Some((144, 0, Some(self.fplmn.clone()))),
            (APDU_READ_RECORD, EF_MSISDN_ID) => Some((144, 0, Some(self.encode_msisdn()))),
            (APDU_READ_BINARY, EF_IMSI_ID) => Some((144, 0, Some(self.encode_imsi()))),
            (APDU_READ_RECORD, EF_MBDN_ID) => {
                let record_idx = (p1 as usize).saturating_sub(1);
                if record_idx < self.mbdn_records.len() {
                    Some((144, 0, Some(self.mbdn_records[record_idx].clone())))
                } else {
                    Some((106, 130, None))
                }
            }
            // Fallback to static profile lookup
            _ => {
                if let Some(resp_str) = self.lookup_simio(command, file_id, p1, p2, p3) {
                    parse_crsm_response_str(&resp_str)
                } else {
                    None
                }
            }
        };

        let (sw1, sw2, data) = crsm_result.unwrap_or((106, 130, None));
        Ok(Some(SimResponse::RestrictedSimAccess { status_byte_1: sw1, status_byte_2: sw2, data }))
    }

    fn handle_open_logical_channel(&mut self, aid: &[u8]) -> SimResult {
        let aid_str = std::str::from_utf8(aid).unwrap_or("");
        let aid_clean = aid_str.trim_matches('"').to_ascii_uppercase();

        if !aid_clean.is_empty() {
            let aid_exists = self.profile.adfs.iter().any(|am| am.aid == aid_clean);
            if !aid_exists {
                info!("[SimService] Rejecting logical channel for unknown AID: {}", aid_clean);
                return Err(ExecutionResult::cme_error(CmeError::NotFound));
            }
        }

        if let Some(channel_idx) = self.logical_channels.iter().position(|&open| !open) {
            self.logical_channels[channel_idx] = true;
            self.selected_aids[channel_idx] = Some(aid_clean);
            Ok(Some(SimResponse::OpenLogicalChannel(channel_idx as u8)))
        } else {
            Err(ExecutionResult::cme_error(CmeError::NoResources))
        }
    }

    fn handle_close_logical_channel(&mut self, channel_id: u8) -> SimResult {
        let idx = channel_id as usize;
        if idx == 0 || idx >= self.logical_channels.len() {
            return Err(ExecutionResult::cme_error(CmeError::InvalidIndex));
        }
        if !self.logical_channels[idx] {
            return Err(ExecutionResult::cme_error(CmeError::NotFound));
        }
        self.logical_channels[idx] = false;
        self.selected_aids[idx] = None;
        self.selected_files[idx] = None;

        // Non-standard: AOSP Goldfish RIL requires "+CCHC" response on channel close to
        // prevent serialization locks.
        // TODO: Extract goldfish-specific quirks into flags.
        Ok(Some(SimResponse::CloseLogicalChannel))
    }

    #[allow(clippy::too_many_arguments)]
    fn process_logical_channel_apdu(
        &mut self,
        idx: usize,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        apdu_data: &[u8],
        cmd_hex: &str,
    ) -> String {
        if cla == CLA_UNSUPPORTED {
            return format_sim_response_hex("+CGLA", "", SW_CLASS_NOT_SUPPORTED);
        }

        // 1. Update state based on APDU instruction (like SELECT)
        if ins == INS_SELECT {
            if matches!(p1, 0x00 | 0x01 | 0x02 | 0x08) && apdu_data.len() >= 2 {
                let fid = ((apdu_data[0] as u16) << 8) | (apdu_data[1] as u16);
                self.selected_files[idx] = Some(fid);
            } else if p1 == 0x04 && !apdu_data.is_empty() {
                self.selected_aids[idx] = Some(hex::encode_upper(apdu_data));
                self.selected_files[idx] = None;
            }
        }

        // 2. Try handling local overrides for specific file reads
        let response_data = match ins {
            INS_READ_BINARY => match self.selected_files[idx] {
                Some(EF_ICCID_ID) => {
                    Some(format_sim_response_hex("+CGLA", &self.iccid, SW_SUCCESS))
                }
                Some(EF_IMSI_ID) => {
                    let imsi_hex = self.encode_imsi();
                    Some(format_sim_response_hex("+CGLA", &imsi_hex, SW_SUCCESS))
                }
                _ => None,
            },
            _ => None,
        };

        // 3. Fallback to XML profile query or default APDU behavior
        match response_data {
            Some(resp) => resp,
            None => {
                if ins == INS_SELECT && p2 == P2_INVALID_SELECT {
                    format_sim_response_hex("+CGLA", "", SW_INCORRECT_PARAMS)
                } else {
                    let active_aid = self.selected_aids[idx].as_deref().unwrap_or("");
                    let selected_fid = self.selected_files[idx];
                    let cmd_hex_upper = cmd_hex.to_ascii_uppercase();
                    if let Some(resp) = self.lookup_cgla(active_aid, selected_fid, &cmd_hex_upper) {
                        format!("+CGLA: {resp}\r\n")
                    } else {
                        match ins {
                            INS_SELECT => {
                                if let Some(fid) = selected_fid
                                    && (find_ef(&self.fs.master_file, &format!("{fid:04X}"))
                                        .is_some()
                                        || find_df(&self.fs.master_file, &format!("{fid:04X}"))
                                            .is_some()
                                        || matches!(
                                            fid,
                                            EF_ICCID_ID
                                                | EF_IMSI_ID
                                                | EF_MSISDN_ID
                                                | EF_FPLMN_ID
                                                | EF_MBDN_ID
                                                | EF_AD_ID
                                        ))
                                {
                                    format_sim_response_hex("+CGLA", "", SW_SUCCESS)
                                } else {
                                    format_sim_response_hex("+CGLA", "", SW_FILE_NOT_FOUND)
                                }
                            }
                            INS_STATUS => {
                                format_sim_response_hex("+CGLA", STATUS_FCP_HEX, SW_SUCCESS)
                            }
                            INS_MANAGE_CHANNEL => match p1 {
                                MANAGE_CHANNEL_ACTION_OPEN => {
                                    if let Some(channel_idx) =
                                        self.logical_channels.iter().position(|&open| !open)
                                    {
                                        self.logical_channels[channel_idx] = true;
                                        let channel_hex = format!("{channel_idx:02X}");
                                        format_sim_response_hex("+CGLA", &channel_hex, SW_SUCCESS)
                                    } else {
                                        format_sim_response_hex(
                                            "+CGLA",
                                            "",
                                            SW_NO_CHANNEL_AVAILABLE,
                                        )
                                    }
                                }
                                MANAGE_CHANNEL_ACTION_CLOSE => {
                                    let target_idx = p2 as usize;
                                    if target_idx == 0 {
                                        format_sim_response_hex("+CGLA", "", SW_INCORRECT_PARAMS) // 6A86
                                    } else if target_idx >= self.logical_channels.len() {
                                        format_sim_response_hex(
                                            "+CGLA",
                                            "",
                                            SW_REFERENCED_DATA_NOT_FOUND,
                                        ) // 6A88
                                    } else if !self.logical_channels[target_idx] {
                                        format_sim_response_hex(
                                            "+CGLA",
                                            "",
                                            SW_NO_CHANNEL_AVAILABLE,
                                        ) // 6A81
                                    } else {
                                        self.logical_channels[target_idx] = false;
                                        self.selected_aids[target_idx] = None;
                                        self.selected_files[target_idx] = None;
                                        format_sim_response_hex("+CGLA", "", SW_SUCCESS)
                                    }
                                }
                                _ => format_sim_response_hex("+CGLA", "", SW_INCORRECT_PARAMS),
                            },
                            _ => {
                                warn!(
                                    "[SimService] Transmit logical channel: command not found in profile: {}, ins: {:02X}",
                                    cmd_hex_upper, ins
                                );
                                format_sim_response_hex("+CGLA", "", SW_INS_NOT_SUPPORTED)
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_transmit_logical_channel(&mut self, channel_id: u8, data: &[u8]) -> SimResult {
        let idx = channel_id as usize;
        if idx >= self.logical_channels.len() {
            return Err(ExecutionResult::cme_error(CmeError::InvalidIndex));
        }
        if !self.logical_channels[idx] {
            return Err(ExecutionResult::cme_error(CmeError::NotFound));
        }

        let data_str = std::str::from_utf8(data).unwrap_or("");
        let data_clean = data_str.trim_matches('"');

        // Parse APDU bytes
        let apdu_bytes = match hex::decode(data_clean) {
            Ok(b) => b,
            Err(_) => {
                return Ok(Some(SimResponse::GenericLogicalChannelAccess(
                    format_sim_response_hex("+CGLA", "", SW_TECHNICAL_PROBLEM),
                )));
            }
        };

        if apdu_bytes.len() < 4 {
            return Ok(Some(SimResponse::GenericLogicalChannelAccess(format_sim_response_hex(
                "+CGLA",
                "",
                SW_TECHNICAL_PROBLEM,
            ))));
        }

        let cla = apdu_bytes[0];
        let ins = apdu_bytes[1];
        let p1 = apdu_bytes[2];
        let p2 = apdu_bytes[3];

        // Parse command data length and data if present
        let mut apdu_data = Vec::new();
        if apdu_bytes.len() > 4 {
            let lc = apdu_bytes[4] as usize;
            if apdu_bytes.len() < 5 + lc {
                return Ok(Some(SimResponse::GenericLogicalChannelAccess(
                    format_sim_response_hex("+CGLA", "", SW_WRONG_LENGTH),
                )));
            }
            apdu_data = apdu_bytes[5..5 + lc].to_vec();
        }

        let response_data =
            self.process_logical_channel_apdu(idx, cla, ins, p1, p2, &apdu_data, data_clean);

        Ok(Some(SimResponse::GenericLogicalChannelAccess(response_data)))
    }

    fn handle_csim_manage_channel(&mut self, p1: u8, p2: u8) -> SimResult {
        if p1 == MANAGE_CHANNEL_ACTION_OPEN {
            // Open channel
            if let Some(channel_idx) = self.logical_channels.iter().position(|&open| !open) {
                self.logical_channels[channel_idx] = true;
                self.selected_aids[channel_idx] = Some("CSIM".to_string()); // CSIM channel
                let resp_hex = format!("{channel_idx:02X}{SW_SUCCESS}");
                Ok(Some(SimResponse::GenericSimAccess(format!("{},{resp_hex}", resp_hex.len()))))
            } else {
                // No channel available
                Ok(Some(SimResponse::GenericSimAccess(format!("4,{SW_NO_CHANNEL_AVAILABLE}"))))
            }
        } else if p1 == MANAGE_CHANNEL_ACTION_CLOSE {
            // Close channel
            let channel_idx = p2 as usize;
            if channel_idx == 0 || channel_idx >= self.logical_channels.len() {
                let status = if channel_idx == 0 {
                    SW_INCORRECT_PARAMS
                } else {
                    SW_REFERENCED_DATA_NOT_FOUND
                };
                Ok(Some(SimResponse::GenericSimAccess(format!("4,{status}"))))
            } else if !self.logical_channels[channel_idx] {
                Ok(Some(SimResponse::GenericSimAccess(format!("4,{SW_NO_CHANNEL_AVAILABLE}"))))
            } else {
                self.logical_channels[channel_idx] = false;
                self.selected_aids[channel_idx] = None;
                self.selected_files[channel_idx] = None;
                Ok(Some(SimResponse::GenericSimAccess(format!("4,{SW_SUCCESS}"))))
            }
        } else {
            Ok(Some(SimResponse::GenericSimAccess(format!("4,{SW_INCORRECT_PARAMS}"))))
        }
    }

    fn handle_generic_sim_access(&mut self, _len: u32, apdu: ApduData) -> SimResult {
        let apdu_str = std::str::from_utf8(apdu.as_ref()).unwrap_or("");
        let apdu_bytes = match hex::decode(apdu_str) {
            Ok(b) => b,
            Err(_) => return Err(ExecutionResult::cme_error(CmeError::Custom(100, "unknown"))),
        };
        if apdu_bytes.len() < 4 {
            return Err(ExecutionResult::cme_error(CmeError::Custom(100, "unknown")));
        }
        let _cla = apdu_bytes[0];
        let ins = apdu_bytes[1];
        let p1 = apdu_bytes[2];
        let p2 = apdu_bytes[3];

        if ins == INS_MANAGE_CHANNEL {
            self.handle_csim_manage_channel(p1, p2)
        } else {
            let cmd_hex_upper = apdu_str.to_ascii_uppercase();
            if let Some(resp) = self.lookup_csim(&cmd_hex_upper) {
                Ok(Some(SimResponse::GenericSimAccess(resp)))
            } else {
                match ins {
                    INS_SELECT => {
                        Ok(Some(SimResponse::GenericSimAccess(format!("4,{SW_SUCCESS}"))))
                    }
                    _ => {
                        info!(
                            "[SimService] Unhandled INS in CSIM: {:02X}, cmd: {}",
                            ins, cmd_hex_upper
                        );
                        Ok(Some(SimResponse::GenericSimAccess(format!("4,{SW_INCORRECT_PARAMS}"))))
                    }
                }
            }
        }
    }

    fn handle_change_password(
        &mut self,
        _facility: QuotedString,
        old_password: QuotedString,
        new_password: QuotedString,
    ) -> SimResult {
        if !(MIN_PIN_LEN..=MAX_PIN_LEN).contains(&old_password.as_ref().len())
            || !(MIN_PIN_LEN..=MAX_PIN_LEN).contains(&new_password.as_ref().len())
        {
            return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword));
        }
        if old_password.as_ref() == self.pin1.as_bytes() {
            self.pin1 = String::from_utf8(new_password.0.to_vec()).unwrap_or_default();
            self.pin1_retries = DEFAULT_PIN_RETRIES;
            Ok(None)
        } else {
            self.pin1_retries = self.pin1_retries.saturating_sub(1);
            if self.pin1_retries == 0 {
                self.state = SimState::PukRequired;
            }
            Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
        }
    }
    fn handle_set_cdma_subscription_source(&mut self, source: u8) -> SimResult {
        self.cdma_subscription_source = source;
        Ok(None)
    }

    fn handle_query_cdma_subscription_source(&self) -> SimResult {
        let source = self.cdma_subscription_source;
        Ok(Some(SimResponse::CdmaSubscriptionSource(source)))
    }

    fn handle_set_cdma_roaming_preference(&mut self, preference: u8) -> SimResult {
        self.cdma_roaming_preference = preference;
        Ok(None)
    }

    fn handle_query_cdma_roaming_preference(&self) -> SimResult {
        let preference = self.cdma_roaming_preference;
        Ok(Some(SimResponse::CdmaRoamingPreference(preference)))
    }

    fn handle_sim_authentication(&self, data: &[u8]) -> SimResult {
        let data_str = std::str::from_utf8(data).unwrap_or("");
        let data_clean = data_str.trim_matches('"');
        let response = match data_clean {
            AUTH_CHALLENGE_1 => AUTH_RESPONSE_1,
            AUTH_CHALLENGE_2 => AUTH_RESPONSE_2,
            AUTH_CHALLENGE_3 => AUTH_RESPONSE_3,
            AUTH_CHALLENGE_4 => AUTH_RESPONSE_4,
            _ => {
                if data_clean.contains(',') {
                    AUTH_RESPONSE_4
                } else {
                    AUTH_RESPONSE_3
                }
            }
        };
        Ok(Some(SimResponse::SimAuthentication(response.to_string())))
    }

    fn handle_update_phone_number(&mut self, phone_number: &[u8]) -> SimResult {
        if let Ok(num_str) = std::str::from_utf8(phone_number) {
            self.msisdn = num_str.to_string();
        }
        Ok(None)
    }

    fn encode_msisdn(&self) -> String {
        let msisdn = &self.msisdn;
        if msisdn.is_empty() {
            return EF_MSISDN_RECORD_FALLBACK.to_string();
        }
        let digits: String = msisdn.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return EF_MSISDN_RECORD_FALLBACK.to_string();
        }

        let ton_npi = if msisdn.starts_with('+') || (digits.len() == 11 && digits.starts_with('1'))
        {
            "91"
        } else {
            "81"
        };

        let swapped = swap_bcd_digits(&digits);
        let bcd_len = 1 + (swapped.len() / 2);
        let bcd_len_hex = format!("{bcd_len:02X}");

        let mut dialing_number = swapped;
        while dialing_number.len() < 20 {
            dialing_number.push_str("FF");
        }
        dialing_number.truncate(20);

        let alpha = "0000000000000000000000000000";
        let suffix = "FFFF";

        format!("{alpha}{bcd_len_hex}{ton_npi}{dialing_number}{suffix}")
    }

    pub fn handle_set_facility_lock(
        &mut self,
        mode: u8,
        passwd: Option<QuotedString>,
    ) -> SimResult {
        if (mode == 0 || mode == 1) && self.state == SimState::PukRequired {
            return Err(ExecutionResult::cme_error(CmeError::SimPukRequired));
        }
        match mode {
            0 => {
                let passwd = match passwd {
                    Some(p) => p,
                    None => return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword)),
                };
                if !(MIN_PIN_LEN..=MAX_PIN_LEN).contains(&passwd.as_ref().len()) {
                    return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword));
                }
                let passwd_str = String::from_utf8(passwd.to_vec()).unwrap_or_default();
                if passwd_str == self.pin1 {
                    self.pin_enabled = false;
                    self.state = SimState::Ready;
                    self.pin1_retries = DEFAULT_PIN_RETRIES;
                    Ok(None)
                } else {
                    self.pin1_retries = self.pin1_retries.saturating_sub(1);
                    if self.pin1_retries == 0 {
                        self.state = SimState::PukRequired;
                    }
                    Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
                }
            }
            1 => {
                let passwd = match passwd {
                    Some(p) => p,
                    None => return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword)),
                };
                if !(MIN_PIN_LEN..=MAX_PIN_LEN).contains(&passwd.as_ref().len()) {
                    return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword));
                }
                let passwd_str = String::from_utf8(passwd.to_vec()).unwrap_or_default();
                if passwd_str == self.pin1 {
                    self.pin_enabled = true;
                    self.state = SimState::Ready;
                    self.pin1_retries = DEFAULT_PIN_RETRIES;
                    Ok(None)
                } else {
                    self.pin1_retries = self.pin1_retries.saturating_sub(1);
                    if self.pin1_retries == 0 {
                        self.state = SimState::PukRequired;
                    }
                    Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
                }
            }
            2 => Ok(Some(SimResponse::FacilityLockStatus(if self.pin_enabled { 1 } else { 0 }))),
            _ => Err(ExecutionResult::Unhandled),
        }
    }

    fn handle_query_pin_retries_spic(&self) -> SimResult {
        let retries = self.pin1_retries;
        Ok(Some(SimResponse::PinRetriesSpic(retries)))
    }

    fn handle_query_pin_retries_cpinr(&self, pin_type: QuotedString) -> SimResult {
        let pin_type_str = std::str::from_utf8(pin_type.as_ref()).unwrap_or("");
        let (retries, default_retries) = match pin_type_str {
            "SIM PIN" => (self.pin1_retries, DEFAULT_PIN_RETRIES),
            "SIM PUK" => (self.puk1_retries, DEFAULT_PUK_RETRIES),
            "SIM PIN2" => (self.pin2_retries, DEFAULT_PIN_RETRIES),
            "SIM PUK2" => (self.puk2_retries, DEFAULT_PUK_RETRIES),
            _ => return Err(ExecutionResult::cme_error(CmeError::IncorrectParameters)),
        };
        Ok(Some(SimResponse::PinRemainingAttempts {
            pin_type: pin_type_str.to_string(),
            retries,
            default_retries,
        }))
    }

    fn encode_imsi(&self) -> String {
        let imsi = &self.imsi;
        if imsi.is_empty() {
            return "083901621032547698".to_string(); // Fallback encoded imsi
        }
        let digits: String = imsi.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return "083901621032547698".to_string();
        }

        #[allow(clippy::manual_is_multiple_of)]
        let is_odd = (digits.len() % 2) != 0;
        let odd_even_indicator = if is_odd { 1 } else { 0 };
        // Identity type for IMSI is 1 (001 binary)
        let identity_type = 1;
        let flags = (odd_even_indicator << 3) | identity_type;

        let first_digit = digits.chars().next().unwrap().to_digit(10).unwrap_or(0) as u8;
        let byte_2 = (first_digit << 4) | flags;

        let remaining_digits = &digits[1..];
        let swapped_remaining = swap_bcd_digits(remaining_digits);

        let encoded_hex = format!("{byte_2:02X}{swapped_remaining}");
        let len_byte = (encoded_hex.len() / 2) as u8;
        format!("{len_byte:02X}{encoded_hex}")
    }

    pub fn execute<'a>(&mut self, command: &SimCommand<'a>) -> ExecutionResult {
        info!("[SimService] Executing SIM command: {:?}", command);
        if self.state == SimState::Absent {
            return ExecutionResult::cme_error(CmeError::SimNotInserted); /* SIM not inserted */
        }
        let sim_result = match command {
            SimCommand::GenericSimAccess(len, apdu) => self.handle_generic_sim_access(*len, *apdu),
            SimCommand::GetSimStatus => self.handle_get_sim_status(),
            SimCommand::EnterPin(pin, new_pin) => self.handle_enter_pin(*pin, *new_pin),
            SimCommand::SimIo { command, file_id, p1, p2, p3, data, path: _ } => {
                let data_str = data.and_then(|d| String::from_utf8(d.0.to_vec()).ok());
                self.handle_sim_io(*command, *file_id, *p1, *p2, *p3, data_str)
            }
            SimCommand::GetImsi => self.handle_get_imsi(),
            SimCommand::GetIccid => self.handle_get_iccid(),
            SimCommand::OpenLogicalChannel(aid) => self.handle_open_logical_channel(aid),
            SimCommand::CloseLogicalChannel(channel_id) => {
                self.handle_close_logical_channel(*channel_id)
            }
            SimCommand::TransmitLogicalChannel(channel_id, _, data) => {
                self.handle_transmit_logical_channel(*channel_id, data)
            }
            SimCommand::ChangePassword(facility, old_password, new_password) => {
                self.handle_change_password(*facility, *old_password, *new_password)
            }
            SimCommand::QueryPinRetries(pin_type) => self.handle_query_pin_retries_cpinr(*pin_type),
            SimCommand::QueryPinRetriesSpic => self.handle_query_pin_retries_spic(),
            SimCommand::SetCdmaSubscriptionSource(source) => {
                self.handle_set_cdma_subscription_source(*source)
            }
            SimCommand::QueryCdmaSubscriptionSource => self.handle_query_cdma_subscription_source(),
            SimCommand::SetCdmaRoamingPreference(preference) => {
                self.handle_set_cdma_roaming_preference(*preference)
            }
            SimCommand::QueryCdmaRoamingPreference => self.handle_query_cdma_roaming_preference(),
            SimCommand::SimAuthentication(data) => self.handle_sim_authentication(data),
            SimCommand::SimAuthenticationVendor(data) => self.handle_sim_authentication(data),
            SimCommand::UpdatePhoneNumber(phone_number) => {
                self.handle_update_phone_number(phone_number)
            }
        };

        sim_result.into()
    }
}

fn find_df<'a>(df: &'a DedicatedFile, id: &str) -> Option<&'a DedicatedFile> {
    if df.file_id == id {
        return Some(df);
    }
    for file in &df.files {
        if let SimFile::Df(df) = file
            && let Some(found) = find_df(df, id)
        {
            return Some(found);
        }
    }
    None
}

fn find_ef<'a>(df: &'a DedicatedFile, id: &str) -> Option<&'a ElementaryFile> {
    for file in &df.files {
        match file {
            SimFile::Ef(ef) => {
                if ef.file_id == id {
                    return Some(ef);
                }
            }
            SimFile::Df(df) => {
                if let Some(ef) = find_ef(df, id) {
                    return Some(ef);
                }
            }
        }
    }
    None
}

fn find_ef_mut<'a>(df: &'a mut DedicatedFile, id: &str) -> Option<&'a mut ElementaryFile> {
    for file in &mut df.files {
        match file {
            SimFile::Ef(ef) => {
                if ef.file_id == id {
                    return Some(ef);
                }
            }
            SimFile::Df(df) => {
                if let Some(ef) = find_ef_mut(df, id) {
                    return Some(ef);
                }
            }
        }
    }
    None
}

fn decode_imsi(bcd_hex: &str) -> Option<String> {
    let bytes = hex::decode(bcd_hex).ok()?;
    if bytes.is_empty() {
        return None;
    }
    if bytes.len() < 2 {
        return None;
    }
    let len = bytes[0] as usize;
    if len == 0 || bytes.len() < 1 + len {
        return None;
    }

    let mut imsi = String::new();
    let digit_1 = bytes[1] >> 4;
    if digit_1 <= 9 {
        imsi.push((b'0' + digit_1) as char);
    }

    for &b in &bytes[2..1 + len] {
        let low = b & 0x0F;
        let high = b >> 4;
        if low <= 9 {
            imsi.push((b'0' + low) as char);
        }
        if high <= 9 {
            imsi.push((b'0' + high) as char);
        }
    }
    Some(imsi)
}

fn decode_msisdn(bcd_hex: &str) -> Option<String> {
    let bytes = hex::decode(bcd_hex).ok()?;
    // Minimum size of EF_MSISDN is 28 bytes (56 hex chars).
    if bytes.len() < 28 {
        return None;
    }
    // Alpha identifier is first 14 bytes.
    // Byte 15 (index 14) is length of BCD number (including TON/NPI).
    let bcd_len = bytes[14] as usize;
    if bcd_len == 0 || bcd_len > 11 {
        return None;
    }
    // Byte 16 (index 15) is TON/NPI.
    let ton_npi = bytes[15];
    let is_international = ton_npi == 0x91;

    let mut msisdn = if is_international { "+".to_string() } else { "".to_string() };

    // Remaining bytes are dialing number
    for &b in &bytes[16..15 + bcd_len] {
        let low = b & 0x0F;
        let high = b >> 4;
        if low <= 9 {
            msisdn.push((b'0' + low) as char);
        }
        if high <= 9 {
            msisdn.push((b'0' + high) as char);
        }
    }
    Some(msisdn)
}

fn swap_bcd_digits(digits: &str) -> String {
    let mut swapped = String::new();
    let chars: Vec<char> = digits.chars().collect();
    for chunk in chars.chunks(2) {
        if chunk.len() == 2 {
            swapped.push(chunk[1]);
            swapped.push(chunk[0]);
        } else if chunk.len() == 1 {
            swapped.push('F');
            swapped.push(chunk[0]);
        }
    }
    swapped
}

fn format_sim_response_hex(prefix: &str, data_hex: &str, sw_hex: &str) -> String {
    let combined = format!("{data_hex}{sw_hex}");
    format!("{prefix}: {},{combined}\r\n", combined.len())
}

fn parse_crsm_response_str(s: &str) -> Option<(u8, u8, Option<String>)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() >= 2 {
        let sw1 = parts[0].parse::<u8>().ok()?;
        let sw2 = parts[1].parse::<u8>().ok()?;
        let data = if parts.len() > 2 { Some(parts[2].trim().to_string()) } else { None };
        Some((sw1, sw2, data))
    } else {
        None
    }
}
