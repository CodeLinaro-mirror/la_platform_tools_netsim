// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, fmt::Write};

use modem_rs_derive::CommandParser;
use tracing::info;

use crate::{
    apdu,
    config::{DedicatedFile, ElementaryFile, FileSystem, SimFile, SimProfile},
    constants::*,
    parser::{ApduData, PinString, QuotedString, parse_raw_data},
    types::{
        CdmaRoamingPreference, CdmaSubscriptionSource, CmeError, DEFAULT_PIN, DEFAULT_PIN2,
        ExecutionResult, FacilityLockMode, Parsable, PhoneNumber,
    },
};

#[derive(Debug, PartialEq, Clone, Copy)]
enum SimAccessType {
    Csim,
    Cgla,
}

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
    SetCdmaSubscriptionSource(CdmaSubscriptionSource),
    /// 3GPP2 C.S0023: Query CDMA subscription source
    #[command(tag = "AT+CCSS?")]
    QueryCdmaSubscriptionSource,
    /// 3GPP2 C.S0023: Set CDMA roaming preference
    #[command(tag = "AT+WRMP=")]
    SetCdmaRoamingPreference(CdmaRoamingPreference),
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
    #[command(tag = "AT+CEID")]
    GetEid,
    #[command(tag = "AT+CATR")]
    GetAtr,
}

const MIN_PIN_LEN: usize = 4;
const MAX_PIN_LEN: usize = 8;
const PUK_LEN: usize = 8;
const DEFAULT_PIN_RETRIES: u32 = 3;
const DEFAULT_PUK_RETRIES: u32 = 10;

const DEFAULT_PUK: &str = "12345678";

const DEFAULT_FALLBACK_IMSI: &str = "310260123456789";
const DEFAULT_FALLBACK_ICCID: &str = "89012608640220133897";
const EF_FPLMN_DATA_FALLBACK: &[u8] = &[0xFF; 12];
const EF_MSISDN_RECORD_FALLBACK: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x91,
    0x51, 0x55, 0x21, 0x43, 0x65, 0xF7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];
const STATUS_FCP_HEX: &str = "62338202782183023F00A50C80016187010183040007DBF08A01058B062F0601020002C60C90016083010183010A83010D8102FFFF";

// SIM Elementary File IDs (EF IDs)
pub(crate) const EF_IMSI_ID: u16 = 0x6F07;
pub(crate) const EF_ICCID_ID: u16 = 0x2FE2;
const EF_FPLMN_ID: u16 = 0x6F7B;
const EF_MSISDN_ID: u16 = 0x6F40;
const EF_MBDN_ID: u16 = 0x6FC7;
const EF_AD_ID: u16 = 0x6FAD;
const EF_FDN_ID: u16 = 0x6F3B;

// SIM file structure constants (TS 51.011 / TS 31.102)
const ADN_FOOTER_LEN: usize = 14; // Length of dialing number info excluding alpha identifier
const BCD_LEN_MAX: usize = 10; // Maximum length of BCD dialing number in ADN/FDN record

// ISO 7816-4 SIM APDU Response Constants
const RESP_SUCCESS: SimResponse = SimResponse::RestrictedSimAccess { sw: SW_SUCCESS, data: None };
const RESP_WRONG_LENGTH: SimResponse =
    SimResponse::RestrictedSimAccess { sw: SW_WRONG_LENGTH, data: None };
const RESP_FILE_NOT_FOUND: SimResponse =
    SimResponse::RestrictedSimAccess { sw: SW_FILE_NOT_FOUND, data: None };
const RESP_INCORRECT_PARAMS: SimResponse =
    SimResponse::RestrictedSimAccess { sw: SW_INCORRECT_PARAMS, data: None };
const RESP_REFERENCED_DATA_NOT_FOUND: SimResponse =
    SimResponse::RestrictedSimAccess { sw: SW_REFERENCED_DATA_NOT_FOUND, data: None };

const P2_INVALID_SELECT: u8 = 0xF0;

// APDU Status Words (SW)

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
    RestrictedSimAccess { sw: u16, data: Option<String> },
    Imsi(String),
    Iccid(String),
    OpenLogicalChannel(u8),
    CloseLogicalChannel,
    GenericLogicalChannelAccess(String),
    GenericSimAccess(String),
    CdmaSubscriptionSource(CdmaSubscriptionSource),
    CdmaRoamingPreference(CdmaRoamingPreference),
    SimAuthentication(String),
    FacilityLockStatus(u8),
    PinRetriesSpic(u32),
    PinRemainingAttempts { pin_type: String, retries: u32, default_retries: u32 },
    Eid(String),
    Atr(String),
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
            SimResponse::RestrictedSimAccess { sw, data } => {
                let sw1 = (sw >> 8) as u8;
                let sw2 = (sw & 0xFF) as u8;
                if let Some(d) = data {
                    write!(f, "+CRSM: {sw1},{sw2},{d}\r\n")
                } else {
                    write!(f, "+CRSM: {sw1},{sw2}\r\n")
                }
            }
            SimResponse::Imsi(imsi) => write!(f, "{imsi}\r\n"),
            SimResponse::Iccid(iccid) => write!(f, "{iccid}\r\n"),
            SimResponse::OpenLogicalChannel(channel_id) => write!(f, "{channel_id}\r\n"),
            SimResponse::CloseLogicalChannel => write!(f, "+CCHC\r\n"),
            SimResponse::GenericLogicalChannelAccess(resp) => write!(f, "+CGLA: {resp}\r\n"),
            SimResponse::GenericSimAccess(resp) => write!(f, "+CSIM: {resp}\r\n"),
            SimResponse::CdmaSubscriptionSource(source) => write!(f, "+CCSS: {source}\r\n"),
            SimResponse::CdmaRoamingPreference(pref) => write!(f, "+WRMP: {pref}\r\n"),
            SimResponse::SimAuthentication(resp) => write!(f, "{resp}"),
            SimResponse::FacilityLockStatus(status) => write!(f, "+CLCK: {status}\r\n"),
            SimResponse::PinRetriesSpic(retries) => write!(f, "+SPIC: {retries}\r\n"),
            SimResponse::PinRemainingAttempts { pin_type, retries, default_retries } => {
                write!(f, "+CPINR: \"{pin_type}\",{retries},{default_retries}\r\n")
            }
            SimResponse::Eid(eid) => write!(f, "+CEID: {eid}\r\n"),
            SimResponse::Atr(atr) => write!(f, "+CATR: {atr}\r\n"),
        }
    }
}

type SimResult = Result<Option<SimResponse>, ExecutionResult>;

// Holds all state related to the SIM card.
pub struct SimService {
    state: SimState,
    pin_enabled: bool,
    pin1: String,
    puk1: String,
    pin1_retries: u32,
    puk1_retries: u32,
    pin2: String,
    pin2_retries: u32,
    puk2_retries: u32,
    fdn_enabled: bool,
    fs: FileSystem,
    sms_messages: HashMap<u8, Vec<u8>>,
    logical_channels: [bool; 4],
    selected_aids: [Option<String>; 4],
    selected_files: [Option<u16>; 4],
    response_buffer: [Vec<u8>; 4],
    cdma_subscription_source: CdmaSubscriptionSource,
    cdma_roaming_preference: CdmaRoamingPreference,
    adfs: Vec<crate::config::ApplicationDedicatedFile>,
    eid: Option<String>,
    atr: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverwritePolicy {
    Always,
    IfUninitialized,
    Never,
}

impl SimService {
    /// Creates a new SimService from a SIM profile configuration.
    pub fn new(profile: &SimProfile) -> Self {
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

        let mut service = Self {
            state,
            pin_enabled,
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
            pin1_retries: profile.pin_profile.pin1_retries.unwrap_or(DEFAULT_PIN_RETRIES),
            puk1_retries: profile.pin_profile.puk1_retries.unwrap_or(DEFAULT_PUK_RETRIES),
            pin2: if !profile.pin_profile.pin2.is_empty() {
                profile.pin_profile.pin2.clone()
            } else {
                DEFAULT_PIN2.to_string()
            },
            pin2_retries: profile.pin_profile.pin2_retries.unwrap_or(DEFAULT_PIN_RETRIES),
            puk2_retries: profile.pin_profile.puk2_retries.unwrap_or(DEFAULT_PUK_RETRIES),
            fdn_enabled: false,
            fs: profile.sim_io.file_system.clone(),
            sms_messages: HashMap::new(),
            // Channel 0 is the basic channel and is always open by default.
            logical_channels: [true, false, false, false],
            selected_aids: [None, None, None, None],
            selected_files: [None; 4],
            response_buffer: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            cdma_subscription_source: CdmaSubscriptionSource::default(),
            cdma_roaming_preference: CdmaRoamingPreference::default(),
            adfs: profile.adfs.clone(),
            eid: profile.eid.clone(),
            atr: profile.atr.clone(),
        };

        // Ensure boot-essential files are present in the filesystem and correctly
        // synchronized
        fn ensure_ef_present(
            df: &mut DedicatedFile,
            id: u16,
            _size: usize,
            record_len: Option<usize>,
            data: Vec<u8>,
            policy: OverwritePolicy,
        ) {
            if let Some(ef) = find_ef_mut(df, id) {
                let should_overwrite = match policy {
                    OverwritePolicy::Always => true,
                    OverwritePolicy::IfUninitialized => ef.data.iter().all(|&b| b == 0xFF),
                    OverwritePolicy::Never => false,
                };
                if should_overwrite {
                    ef.data = data;
                    ef.record_len = record_len;
                }
            } else {
                df.files.push(SimFile::ElementaryFile(ElementaryFile {
                    file_id: id,
                    record_len,
                    data,
                }));
            }
        }

        let iccid_swapped = crate::pdu::bcd::string_to_bcd(&iccid);
        let imsi_encoded = crate::pdu::bcd::encode_imsi(&imsi);
        let msisdn_encoded = encode_msisdn_str(&msisdn);
        let fplmn_data = EF_FPLMN_DATA_FALLBACK.to_vec();

        ensure_ef_present(
            &mut service.fs.master_file,
            EF_ICCID_ID,
            10,
            None,
            iccid_swapped,
            OverwritePolicy::Always,
        );
        if let Some(encoded) = imsi_encoded {
            ensure_ef_present(
                &mut service.fs.master_file,
                EF_IMSI_ID,
                9,
                None,
                encoded,
                OverwritePolicy::Always,
            );
        }
        ensure_ef_present(
            &mut service.fs.master_file,
            EF_MSISDN_ID,
            28,
            Some(28),
            msisdn_encoded,
            OverwritePolicy::IfUninitialized,
        );
        ensure_ef_present(
            &mut service.fs.master_file,
            EF_FPLMN_ID,
            12,
            None,
            fplmn_data,
            OverwritePolicy::Never,
        );
        ensure_ef_present(
            &mut service.fs.master_file,
            EF_MBDN_ID,
            152,
            Some(38),
            vec![0xFF; 152],
            OverwritePolicy::Never,
        );

        service
    }

    fn lookup_cgla(
        &self,
        active_aid: &str,
        file_id: Option<u16>,
        apdu: &apdu::ParsedApdu<'_>,
    ) -> Option<String> {
        let is_select_aid = apdu.ins == apdu::Instruction::Select && apdu.p1 == 0x04;

        let aid =
            if is_select_aid { hex::encode_upper(apdu.data()) } else { active_aid.to_string() };

        let adf = self.adfs.iter().find(|am| am.aid == aid)?;

        if !is_select_aid
            && let Some(fid) = file_id
            && let Some(file_mock) = adf.files.iter().find(|f| f.id == fid)
            && let Some(m) = file_mock.cgla.iter().find(|m| m.cmd.matches(apdu))
        {
            return Some(m.response.to_string());
        }

        if apdu.ins == apdu::Instruction::Select
            && (apdu.p1 == 0x00 || apdu.p1 == 0x08)
            && apdu.data().len() >= 2
        {
            let target_fid = ((apdu.data()[0] as u16) << 8) | (apdu.data()[1] as u16);
            if let Some(file_mock) = adf.files.iter().find(|f| f.id == target_fid)
                && let Some(m) = file_mock.cgla.iter().find(|m| m.cmd.matches(apdu))
            {
                return Some(m.response.to_string());
            }
        }

        adf.cgla.iter().find(|m| m.cmd.matches(apdu)).map(|m| m.response.to_string())
    }

    fn lookup_csim(&self, apdu: &apdu::ParsedApdu<'_>) -> Option<String> {
        let adf = self.adfs.iter().find(|am| am.aid == "CSIM")?;
        adf.csim.iter().find(|m| m.cmd.matches(apdu)).map(|m| m.response.to_string())
    }

    fn get_imsi(&self) -> String {
        find_ef(&self.fs.master_file, EF_IMSI_ID)
            .and_then(|ef| decode_imsi(&ef.data))
            .unwrap_or_else(|| DEFAULT_FALLBACK_IMSI.to_string())
    }

    fn get_iccid(&self) -> String {
        find_ef(&self.fs.master_file, EF_ICCID_ID)
            .map(|ef| crate::pdu::bcd::bcd_to_string(&ef.data))
            .unwrap_or_else(|| DEFAULT_FALLBACK_ICCID.to_string())
    }

    pub(crate) fn get_msisdn(&self) -> Option<PhoneNumber> {
        let raw =
            find_ef(&self.fs.master_file, EF_MSISDN_ID).and_then(|ef| decode_msisdn(&ef.data))?;
        PhoneNumber::parse(raw.as_bytes()).map(|(_, p)| p).ok()
    }

    pub fn set_msisdn(&mut self, msisdn: &str) {
        let encoded = encode_msisdn_str(msisdn);
        if let Some(ef) = find_ef_mut(&mut self.fs.master_file, EF_MSISDN_ID) {
            ef.data = encoded;
        }
    }

    fn select_sim_file(&mut self, idx: usize, apdu: &apdu::ParsedApdu<'_>) -> Result<(), u16> {
        let Ok(p1) = apdu::SelectP1::try_from(apdu.p1) else {
            return Err(SW_INCORRECT_PARAMS);
        };
        match p1 {
            apdu::SelectP1::ByDfName => {
                if apdu.data().is_empty() {
                    return Err(SW_FILE_NOT_FOUND);
                }
                let aid_upper = hex::encode_upper(apdu.data());
                if self.adfs.iter().any(|am| am.aid == aid_upper) {
                    self.selected_aids[idx] = Some(aid_upper);
                    self.selected_files[idx] = None;
                    Ok(())
                } else {
                    Err(SW_FILE_NOT_FOUND)
                }
            }
            apdu::SelectP1::ByFid | apdu::SelectP1::ByPathFromMf => {
                if apdu.data().len() != 2 {
                    return Err(SW_WRONG_LENGTH);
                }
                let fid = ((apdu.data()[0] as u16) << 8) | (apdu.data()[1] as u16);
                let is_file_in_active_adf = if let Some(active_aid) = &self.selected_aids[idx]
                    && let Some(adf) = self.adfs.iter().find(|a| a.aid == *active_aid)
                {
                    let in_overrides = adf.files.iter().any(|f| f.id == fid);
                    let in_fs_subtree =
                        if let Some(adf_df) = find_df(&self.fs.master_file, ADF_DEFAULT_FILE_ID) {
                            find_ef(adf_df, fid).is_some() || find_df(adf_df, fid).is_some()
                        } else {
                            false
                        };
                    in_overrides || in_fs_subtree
                } else {
                    false
                };
                if find_ef(&self.fs.master_file, fid).is_some()
                    || find_df(&self.fs.master_file, fid).is_some()
                    || is_file_in_active_adf
                    || matches!(
                        fid,
                        EF_ICCID_ID
                            | EF_IMSI_ID
                            | EF_MSISDN_ID
                            | EF_FPLMN_ID
                            | EF_MBDN_ID
                            | EF_AD_ID
                    )
                {
                    self.selected_files[idx] = Some(fid);
                    if !is_file_in_active_adf {
                        self.selected_aids[idx] = None;
                    }
                    Ok(())
                } else {
                    Err(SW_FILE_NOT_FOUND)
                }
            }
        }
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
        Ok(Some(SimResponse::Imsi(self.get_imsi())))
    }

    fn handle_get_iccid(&self) -> SimResult {
        Ok(Some(SimResponse::Iccid(self.get_iccid())))
    }

    fn handle_get_eid(&self) -> SimResult {
        if let Some(eid) = &self.eid {
            Ok(Some(SimResponse::Eid(eid.clone())))
        } else {
            Err(ExecutionResult::cme_error(CmeError::NotFound))
        }
    }

    fn handle_get_atr(&self) -> SimResult {
        if let Some(atr) = &self.atr {
            Ok(Some(SimResponse::Atr(atr.clone())))
        } else {
            Err(ExecutionResult::cme_error(CmeError::NotFound))
        }
    }

    fn update_sim_file(
        &mut self,
        command: apdu::Instruction,
        file_id: u16,
        p1: u8,
        p2: u8,
        hex_str: &str,
    ) -> Result<(), SimResponse> {
        // Update in the file system if it exists there
        if let Some(ef) = find_ef_mut(&mut self.fs.master_file, file_id) {
            if command == apdu::Instruction::UpdateBinary {
                let offset = ((p1 as usize) << 8) | (p2 as usize);
                let Ok(new_bytes) = hex::decode(hex_str) else {
                    return Err(RESP_INCORRECT_PARAMS);
                };
                if offset + new_bytes.len() > ef.data.len() {
                    ef.data.resize(offset + new_bytes.len(), 0xFF);
                }
                ef.data[offset..offset + new_bytes.len()].copy_from_slice(&new_bytes);
                Ok(())
            } else if command == apdu::Instruction::UpdateRecord {
                // UPDATE RECORD
                if let Ok(mode) = apdu::RecordMode::try_from(p2) {
                    if mode != apdu::RecordMode::AbsoluteMode {
                        return Err(RESP_INCORRECT_PARAMS);
                    }
                    let Ok(new_bytes) = hex::decode(hex_str) else {
                        return Err(RESP_INCORRECT_PARAMS);
                    };
                    if let Some(rec_len) = ef.record_len
                        && rec_len > 0
                    {
                        if p1 == 0 {
                            return Err(RESP_INCORRECT_PARAMS);
                        }
                        if new_bytes.len() != rec_len {
                            return Err(RESP_WRONG_LENGTH);
                        }
                        let record_num = p1 as usize;
                        let start = (record_num - 1) * rec_len;
                        let end = start + rec_len;
                        if end > ef.data.len() {
                            return Err(RESP_REFERENCED_DATA_NOT_FOUND);
                        }
                        ef.data[start..end].copy_from_slice(&new_bytes);
                        return Ok(());
                    }
                    ef.data = new_bytes;
                    Ok(())
                } else {
                    Err(RESP_INCORRECT_PARAMS)
                }
            } else {
                Err(RESP_INCORRECT_PARAMS)
            }
        } else if matches!(
            file_id,
            EF_ICCID_ID | EF_IMSI_ID | EF_MSISDN_ID | EF_FPLMN_ID | EF_MBDN_ID | EF_AD_ID
        ) {
            Ok(())
        } else {
            Err(RESP_FILE_NOT_FOUND)
        }
    }

    fn read_binary_from_fs(
        &self,
        file_id: u16,
        p1: u8,
        p2: u8,
        p3: u8,
    ) -> Result<String, SimResponse> {
        if let Some(ef) = find_ef(&self.fs.master_file, file_id) {
            let offset = ((p1 as usize) << 8) | (p2 as usize);
            let ef_size = ef.size();
            if offset > ef_size {
                return Err(RESP_INCORRECT_PARAMS);
            }
            let length = if p3 == 0 { ef_size - offset } else { p3 as usize };
            if offset + length > ef_size {
                return Err(RESP_WRONG_LENGTH);
            }
            let end = std::cmp::min(offset + length, ef.data.len());
            let sliced = if offset < ef.data.len() { &ef.data[offset..end] } else { &[] };
            Ok(hex::encode_upper(sliced))
        } else {
            Err(RESP_FILE_NOT_FOUND)
        }
    }

    fn read_record_from_fs(
        &self,
        file_id: u16,
        record_num: u8,
        p2: u8,
    ) -> Result<String, SimResponse> {
        if let Ok(mode) = apdu::RecordMode::try_from(p2) {
            if mode != apdu::RecordMode::AbsoluteMode {
                return Err(RESP_INCORRECT_PARAMS);
            }
            if let Some(ef) = find_ef(&self.fs.master_file, file_id) {
                if let Some(rec_len) = ef.record_len
                    && rec_len > 0
                {
                    let record_num = record_num as usize;
                    if record_num > 0 {
                        let start = (record_num - 1) * rec_len;
                        let end = start + rec_len;
                        if end <= ef.data.len() {
                            return Ok(hex::encode_upper(&ef.data[start..end]));
                        }
                    }
                    return Err(RESP_REFERENCED_DATA_NOT_FOUND);
                }
                Err(RESP_REFERENCED_DATA_NOT_FOUND)
            } else {
                Err(RESP_FILE_NOT_FOUND)
            }
        } else {
            Err(RESP_INCORRECT_PARAMS)
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
        let ins = apdu::Instruction::from(command as u8);
        // 1. Handle UPDATE BINARY and UPDATE RECORD
        if ins == apdu::Instruction::UpdateBinary || ins == apdu::Instruction::UpdateRecord {
            let resp = if let Some(hex_str) = data {
                match self.update_sim_file(ins, file_id, p1, p2, &hex_str) {
                    Ok(()) => RESP_SUCCESS,
                    Err(err_resp) => err_resp,
                }
            } else {
                RESP_INCORRECT_PARAMS
            };
            return Ok(Some(resp));
        }

        // 2. Try to read from the loaded FileSystem first (for READ BINARY and SELECT)
        if ins == apdu::Instruction::ReadBinary {
            match self.read_binary_from_fs(file_id, p1, p2, p3) {
                Ok(data_hex) => {
                    return Ok(Some(SimResponse::RestrictedSimAccess {
                        sw: SW_SUCCESS,
                        data: Some(data_hex),
                    }));
                }
                Err(err_resp) => return Ok(Some(err_resp)),
            }
        } else if ins == apdu::Instruction::ReadRecord {
            match self.read_record_from_fs(file_id, p1, p2) {
                Ok(record_hex) => {
                    return Ok(Some(SimResponse::RestrictedSimAccess {
                        sw: SW_SUCCESS,
                        data: Some(record_hex),
                    }));
                }
                Err(err_resp) => return Ok(Some(err_resp)),
            }
        } else if ins == apdu::Instruction::Select
            && find_df(&self.fs.master_file, file_id).is_some()
        {
            return Ok(Some(SimResponse::RestrictedSimAccess {
                sw: SW_SUCCESS,
                data: Some("6210".to_string()),
            }));
        } else if ins == apdu::Instruction::Status {
            // Return FCP template for Master File (MF)
            return Ok(Some(SimResponse::RestrictedSimAccess {
                sw: SW_SUCCESS,
                data: Some(STATUS_FCP_HEX.to_string()),
            }));
        }

        Ok(Some(RESP_FILE_NOT_FOUND))
    }

    fn handle_open_logical_channel(&mut self, aid: &[u8]) -> SimResult {
        let aid_str = std::str::from_utf8(aid).unwrap_or("");
        let aid_clean = aid_str.trim_matches('"').to_ascii_uppercase();

        if !aid_clean.is_empty() {
            let aid_exists = self.adfs.iter().any(|am| am.aid == aid_clean);
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

    fn process_logical_channel_apdu(
        &mut self,
        access_type: SimAccessType,
        idx: usize,
        apdu: &apdu::ParsedApdu<'_>,
    ) -> String {
        if !apdu.class.is_supported() {
            return format_sim_payload_status(SW_CLASS_NOT_SUPPORTED);
        }

        let active_aid = self.selected_aids[idx].as_deref();
        let selected_fid = self.selected_files[idx];

        if apdu.ins != apdu::Instruction::GetResponse {
            self.response_buffer[idx].clear();
        }

        // 2. Immediate validation check for invalid SELECT P2 parameter
        if apdu.ins == apdu::Instruction::Select && apdu.p2 == P2_INVALID_SELECT {
            return format_sim_payload_status(SW_INCORRECT_PARAMS);
        }

        // 3. Honor explicit APDU mappings defined in the XML profile
        let profile_response = match access_type {
            SimAccessType::Cgla => self.lookup_cgla(active_aid.unwrap_or(""), selected_fid, apdu),
            SimAccessType::Csim => self.lookup_csim(apdu),
        };
        if let Some(resp) = profile_response {
            if apdu.ins == apdu::Instruction::Select {
                if let Some(sw) = get_status_word_from_payload(&resp)
                    && !is_error_sw(sw)
                {
                    // Ignore any error because the explicit XML profile
                    // mapping is authoritative
                    let _ = self.select_sim_file(idx, apdu);
                }
            } else if apdu.ins == apdu::Instruction::ManageChannel
                && let Some(sw) = get_status_word_from_payload(&resp)
                && sw == SW_SUCCESS
                && let Ok(action) = apdu::ManageChannelAction::try_from(apdu.p1)
            {
                match action {
                    apdu::ManageChannelAction::Open => {
                        if let Some(channel_idx) = get_channel_idx_from_open_response(&resp)
                            && channel_idx < self.logical_channels.len()
                        {
                            self.logical_channels[channel_idx] = true;
                            self.selected_files[channel_idx] = Some(MF_FILE_ID);
                            if access_type == SimAccessType::Csim {
                                self.selected_aids[channel_idx] = Some("CSIM".to_string());
                            }
                        }
                    }
                    apdu::ManageChannelAction::Close => {
                        let target_idx = if apdu.p2 == 0 { idx } else { apdu.p2 as usize };
                        if target_idx != 0 && target_idx < self.logical_channels.len() {
                            self.logical_channels[target_idx] = false;
                            self.selected_aids[target_idx] = None;
                            self.selected_files[target_idx] = None;
                            self.response_buffer[target_idx].clear();
                        }
                    }
                }
            }
            return resp;
        }

        // 4. Process standard filesystem APDUs
        let response_data = match apdu.ins {
            apdu::Instruction::ReadBinary => {
                if let Some(fid) = selected_fid {
                    match self.read_binary_from_fs(
                        fid,
                        apdu.p1,
                        apdu.p2,
                        apdu.expected_length().unwrap_or(0),
                    ) {
                        Ok(data_hex) => Some(format_sim_payload_data(&data_hex, SW_SUCCESS)),
                        Err(SimResponse::RestrictedSimAccess { sw, .. }) => {
                            Some(format_sim_payload_status(sw))
                        }
                        Err(_) => Some(format_sim_payload_status(SW_TECHNICAL_PROBLEM)),
                    }
                } else {
                    None
                }
            }
            apdu::Instruction::ReadRecord => {
                if let Some(fid) = selected_fid {
                    match self.read_record_from_fs(fid, apdu.p1, apdu.p2) {
                        Ok(record_hex) => Some(format_sim_payload_data(&record_hex, SW_SUCCESS)),
                        Err(SimResponse::RestrictedSimAccess { sw, .. }) => {
                            Some(format_sim_payload_status(sw))
                        }
                        Err(_) => Some(format_sim_payload_status(SW_TECHNICAL_PROBLEM)),
                    }
                } else {
                    None
                }
            }
            apdu::Instruction::UpdateBinary => {
                if let Some(fid) = selected_fid {
                    let data_hex = hex::encode_upper(apdu.data());
                    match self.update_sim_file(
                        apdu::Instruction::UpdateBinary,
                        fid,
                        apdu.p1,
                        apdu.p2,
                        &data_hex,
                    ) {
                        Ok(()) => Some(format_sim_payload_status(SW_SUCCESS)),
                        Err(SimResponse::RestrictedSimAccess { sw, .. }) => {
                            Some(format_sim_payload_status(sw))
                        }
                        Err(_) => Some(format_sim_payload_status(SW_TECHNICAL_PROBLEM)),
                    }
                } else {
                    None
                }
            }
            apdu::Instruction::UpdateRecord => {
                if let Some(fid) = selected_fid {
                    let data_hex = hex::encode_upper(apdu.data());
                    match self.update_sim_file(
                        apdu::Instruction::UpdateRecord,
                        fid,
                        apdu.p1,
                        apdu.p2,
                        &data_hex,
                    ) {
                        Ok(()) => Some(format_sim_payload_status(SW_SUCCESS)),
                        Err(SimResponse::RestrictedSimAccess { sw, .. }) => {
                            Some(format_sim_payload_status(sw))
                        }
                        Err(_) => Some(format_sim_payload_status(SW_TECHNICAL_PROBLEM)),
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        // 5. Handle basic control APDUs or return error
        match response_data {
            Some(resp) => resp,
            None => match apdu.ins {
                apdu::Instruction::Select => {
                    if let Ok(p2) = apdu::SelectP2::try_from(apdu.p2) {
                        if p2 == apdu::SelectP2::ReturnProprietary {
                            return format_sim_payload_status(SW_INCORRECT_PARAMS);
                        }
                        match self.select_sim_file(idx, apdu) {
                            Ok(()) => {
                                if p2 == apdu::SelectP2::ReturnFcp {
                                    let fid =
                                        self.selected_files[idx].unwrap_or(ADF_DEFAULT_FILE_ID);
                                    let fcp_hex =
                                        generate_df_fcp(fid, self.selected_aids[idx].as_deref());
                                    if let Ok(fcp_bytes) = hex::decode(&fcp_hex) {
                                        let len = fcp_bytes.len();
                                        self.response_buffer[idx] = fcp_bytes;
                                        let sw_low = if len >= 256 { 0 } else { len as u8 };
                                        let sw = ((SW_BYTES_REMAINING_PREFIX as u16) << 8)
                                            | (sw_low as u16);
                                        return format_sim_payload_status(sw);
                                    }
                                }
                                format_sim_payload_status(SW_SUCCESS)
                            }
                            Err(sw) => format_sim_payload_status(sw),
                        }
                    } else {
                        format_sim_payload_status(SW_INCORRECT_PARAMS)
                    }
                }
                apdu::Instruction::Status => format_sim_payload_data(STATUS_FCP_HEX, SW_SUCCESS),
                apdu::Instruction::ManageChannel => {
                    let Ok(action) = apdu::ManageChannelAction::try_from(apdu.p1) else {
                        return format_sim_payload_status(SW_INCORRECT_PARAMS);
                    };
                    match action {
                        apdu::ManageChannelAction::Open => {
                            if let Some(channel_idx) =
                                self.logical_channels.iter().position(|&open| !open)
                            {
                                self.logical_channels[channel_idx] = true;
                                let channel_hex = format!("{channel_idx:02X}");
                                format_sim_payload_data(&channel_hex, SW_SUCCESS)
                            } else {
                                format_sim_payload_status(SW_NO_CHANNEL_AVAILABLE)
                            }
                        }
                        apdu::ManageChannelAction::Close => {
                            let target_idx = if apdu.p2 == 0 { idx } else { apdu.p2 as usize };
                            if target_idx == 0 {
                                format_sim_payload_status(SW_INCORRECT_PARAMS) // 6A86
                            } else if target_idx >= self.logical_channels.len()
                                || !self.logical_channels[target_idx]
                            {
                                format_sim_payload_status(SW_REFERENCED_DATA_NOT_FOUND) // 6A88
                            } else {
                                self.logical_channels[target_idx] = false;
                                self.selected_aids[target_idx] = None;
                                self.selected_files[target_idx] = None;
                                self.response_buffer[target_idx].clear();
                                format_sim_payload_status(SW_SUCCESS)
                            }
                        }
                    }
                }
                apdu::Instruction::GetResponse => self.handle_get_response(idx, apdu),
                _ => {
                    if access_type == SimAccessType::Cgla {
                        format_sim_payload_status(SW_INS_NOT_SUPPORTED)
                    } else {
                        format_sim_payload_status(SW_INCORRECT_PARAMS)
                    }
                }
            },
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

        let apdu_bytes = match hex::decode(data_clean) {
            Ok(b) => b,
            Err(_) => {
                return Ok(Some(SimResponse::GenericLogicalChannelAccess(
                    format_sim_payload_status(SW_TECHNICAL_PROBLEM),
                )));
            }
        };

        let parsed = match apdu::ParsedApdu::parse(&apdu_bytes) {
            Ok(p) => p,
            Err(sw) => {
                return Ok(Some(SimResponse::GenericLogicalChannelAccess(
                    format_sim_payload_status(sw),
                )));
            }
        };

        if !parsed.class.is_supported()
            || (parsed.class.channel() != 0 && parsed.class.channel() != idx)
        {
            return Ok(Some(SimResponse::GenericLogicalChannelAccess(format_sim_payload_status(
                SW_CLASS_NOT_SUPPORTED,
            ))));
        }

        let response_data = self.process_logical_channel_apdu(SimAccessType::Cgla, idx, &parsed);

        Ok(Some(SimResponse::GenericLogicalChannelAccess(response_data)))
    }

    fn handle_csim_manage_channel(&mut self, channel_idx: usize, p1: u8, p2: u8) -> SimResult {
        let Ok(action) = apdu::ManageChannelAction::try_from(p1) else {
            let status = SW_INCORRECT_PARAMS;
            return Ok(Some(SimResponse::GenericSimAccess(format!("4,{status:04X}"))));
        };
        match action {
            apdu::ManageChannelAction::Open => {
                if let Some(channel_idx) = self.logical_channels.iter().position(|&open| !open) {
                    self.logical_channels[channel_idx] = true;
                    self.selected_aids[channel_idx] = Some("CSIM".to_string()); // CSIM channel
                    let resp_hex = format!("{channel_idx:02X}{SW_SUCCESS:04X}");
                    Ok(Some(SimResponse::GenericSimAccess(format!(
                        "{},{resp_hex}",
                        resp_hex.len()
                    ))))
                } else {
                    // No channel available
                    Ok(Some(SimResponse::GenericSimAccess(format!(
                        "4,{SW_NO_CHANNEL_AVAILABLE:04X}"
                    ))))
                }
            }
            apdu::ManageChannelAction::Close => {
                let target_idx = if p2 == 0 { channel_idx } else { p2 as usize };
                if target_idx == 0 {
                    let status = SW_INCORRECT_PARAMS;
                    Ok(Some(SimResponse::GenericSimAccess(format!("4,{status:04X}"))))
                } else if target_idx >= self.logical_channels.len()
                    || !self.logical_channels[target_idx]
                {
                    let status = SW_REFERENCED_DATA_NOT_FOUND;
                    Ok(Some(SimResponse::GenericSimAccess(format!("4,{status:04X}"))))
                } else {
                    self.logical_channels[target_idx] = false;
                    self.selected_aids[target_idx] = None;
                    self.selected_files[target_idx] = None;
                    Ok(Some(SimResponse::GenericSimAccess(format!("4,{SW_SUCCESS:04X}"))))
                }
            }
        }
    }

    fn handle_generic_sim_access(&mut self, _len: u32, apdu: ApduData) -> SimResult {
        let apdu_str = std::str::from_utf8(apdu.as_ref()).unwrap_or("");
        let apdu_bytes = match hex::decode(apdu_str) {
            Ok(b) => b,
            Err(_) => return Err(ExecutionResult::cme_error(CmeError::Custom(100, "unknown"))),
        };

        let parsed = match apdu::ParsedApdu::parse(&apdu_bytes) {
            Ok(p) => p,
            Err(sw) => {
                let payload = format_sim_payload_status(sw);
                return Ok(Some(SimResponse::GenericSimAccess(payload)));
            }
        };

        let channel_idx = parsed.class.channel();
        if !parsed.class.is_supported()
            || channel_idx >= self.logical_channels.len()
            || !self.logical_channels[channel_idx]
        {
            return Ok(Some(SimResponse::GenericSimAccess(format_sim_payload_status(
                SW_CLASS_NOT_SUPPORTED,
            ))));
        }

        if parsed.ins == apdu::Instruction::ManageChannel {
            self.handle_csim_manage_channel(channel_idx, parsed.p1, parsed.p2)
        } else {
            let payload =
                self.process_logical_channel_apdu(SimAccessType::Csim, channel_idx, &parsed);
            Ok(Some(SimResponse::GenericSimAccess(payload)))
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
    fn handle_set_cdma_subscription_source(&mut self, source: CdmaSubscriptionSource) -> SimResult {
        self.cdma_subscription_source = source;
        Ok(None)
    }

    fn handle_query_cdma_subscription_source(&self) -> SimResult {
        let source = self.cdma_subscription_source;
        Ok(Some(SimResponse::CdmaSubscriptionSource(source)))
    }

    fn handle_set_cdma_roaming_preference(
        &mut self,
        preference: CdmaRoamingPreference,
    ) -> SimResult {
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
            self.set_msisdn(num_str);
        }
        Ok(None)
    }

    pub fn handle_set_facility_lock(
        &mut self,
        mode: FacilityLockMode,
        passwd: Option<QuotedString>,
    ) -> SimResult {
        if (mode == FacilityLockMode::Unlock || mode == FacilityLockMode::Lock)
            && self.state == SimState::PukRequired
        {
            return Err(ExecutionResult::cme_error(CmeError::SimPukRequired));
        }
        match mode {
            FacilityLockMode::Unlock => {
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
            FacilityLockMode::Lock => {
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
            FacilityLockMode::QueryStatus => {
                Ok(Some(SimResponse::FacilityLockStatus(if self.pin_enabled { 1 } else { 0 })))
            }
        }
    }

    pub(crate) fn handle_set_fdn_lock(
        &mut self,
        mode: FacilityLockMode,
        passwd: Option<QuotedString>,
    ) -> SimResult {
        if (mode == FacilityLockMode::Unlock || mode == FacilityLockMode::Lock)
            && self.pin2_retries == 0
        {
            return Err(ExecutionResult::cme_error(CmeError::SimPuk2Required));
        }

        match mode {
            FacilityLockMode::Unlock | FacilityLockMode::Lock => {
                let passwd = match passwd {
                    Some(p) => p,
                    None => return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword)),
                };
                if !(MIN_PIN_LEN..=MAX_PIN_LEN).contains(&passwd.as_ref().len()) {
                    return Err(ExecutionResult::cme_error(CmeError::IncorrectPassword));
                }
                if passwd.as_ref() == self.pin2.as_bytes() {
                    self.fdn_enabled = mode == FacilityLockMode::Lock;
                    self.pin2_retries = DEFAULT_PIN_RETRIES;
                    Ok(None)
                } else {
                    self.pin2_retries = self.pin2_retries.saturating_sub(1);
                    Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
                }
            }
            FacilityLockMode::QueryStatus => {
                Ok(Some(SimResponse::FacilityLockStatus(if self.fdn_enabled { 1 } else { 0 })))
            }
        }
    }

    pub(crate) fn is_fdn_allowed(&self, number: &PhoneNumber) -> bool {
        if !self.fdn_enabled {
            return true;
        }

        let Some(fdn_ef) = find_ef(&self.fs.master_file, EF_FDN_ID) else {
            return false;
        };

        let record_len = fdn_ef.record_len.unwrap_or(28);
        if record_len < ADN_FOOTER_LEN {
            return false;
        }

        let normalized_target = number.normalized();

        for record in fdn_ef.data.chunks(record_len) {
            if record.len() < record_len {
                continue;
            }
            if record.iter().all(|&b| b == 0xFF) {
                continue;
            }

            let len_offset = record_len - ADN_FOOTER_LEN;
            let len_byte = record[len_offset];
            if len_byte == 0xFF || len_byte == 0 {
                continue;
            }

            let bcd_len = (len_byte as usize).saturating_sub(1).min(BCD_LEN_MAX);
            if bcd_len == 0 {
                continue;
            }

            let bcd_start = len_offset + 2;
            let bcd_bytes = &record[bcd_start..bcd_start + bcd_len];

            let decoded_number = crate::pdu::bcd::bcd_to_string(bcd_bytes);
            if decoded_number.is_empty() {
                continue;
            }

            if normalized_target.starts_with(&decoded_number) {
                return true;
            }
        }

        false
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

    pub fn execute<'a>(&mut self, command: &SimCommand<'a>) -> ExecutionResult {
        info!("[SimService] Executing SIM command: {command:?}");
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
            SimCommand::GetEid => self.handle_get_eid(),
            SimCommand::GetAtr => self.handle_get_atr(),
        };

        sim_result.into()
    }

    fn handle_get_response(&mut self, idx: usize, apdu: &apdu::ParsedApdu<'_>) -> String {
        if apdu.p1 != 0x00 || apdu.p2 != 0x00 {
            return format_sim_payload_status(SW_INCORRECT_PARAMS);
        }
        if self.response_buffer[idx].is_empty() {
            return format_sim_payload_status(SW_REFERENCED_DATA_NOT_FOUND);
        }

        let req_len = if let Some(le) = apdu.expected_length() {
            if le == 0 { 256 } else { le as usize }
        } else {
            self.response_buffer[idx].len()
        };

        let available = self.response_buffer[idx].len();
        let take_len = req_len.min(available);
        let chunk: Vec<u8> = self.response_buffer[idx].drain(..take_len).collect();
        let remaining = self.response_buffer[idx].len();
        let hex_data = hex::encode_upper(&chunk);

        if remaining > 0 {
            let sw_low = if remaining >= 256 { 0 } else { remaining as u8 };
            let sw = ((SW_BYTES_REMAINING_PREFIX as u16) << 8) | (sw_low as u16);
            format_sim_payload_data(&hex_data, sw)
        } else {
            format_sim_payload_data(&hex_data, SW_SUCCESS)
        }
    }
}

fn encode_msisdn_str(msisdn: &str) -> Vec<u8> {
    if msisdn.is_empty() {
        return EF_MSISDN_RECORD_FALLBACK.to_vec();
    }
    let digits: String = msisdn.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return EF_MSISDN_RECORD_FALLBACK.to_vec();
    }

    let ton_npi = if msisdn.starts_with('+') || (digits.len() == 11 && digits.starts_with('1')) {
        0x91
    } else {
        0x81
    };

    let swapped_bytes = crate::pdu::bcd::string_to_bcd(&digits);
    let bcd_len = (1 + swapped_bytes.len()) as u8;

    let mut result = Vec::with_capacity(28);
    result.resize(14, 0x00);
    result.push(bcd_len);
    result.push(ton_npi);

    let mut dialing = swapped_bytes;
    dialing.resize(10, 0xFF);
    result.extend(dialing);

    result.extend_from_slice(&[0xFF, 0xFF]);

    result
}

fn generate_df_fcp(df_id: u16, active_aid: Option<&str>) -> String {
    let mut fcp_bytes = hex::decode(STATUS_FCP_HEX).unwrap();
    // Overwrite File ID in FCP template (Tag '83' at index 6: 83 02 3F 00)
    fcp_bytes[8] = ((df_id >> 8) & 0xFF) as u8;
    fcp_bytes[9] = (df_id & 0xFF) as u8;

    if df_id == ADF_DEFAULT_FILE_ID
        && let Some(aid) = active_aid
        && let Ok(aid_bytes) = hex::decode(aid)
    {
        // Append Tag '84' (DF Name / AID)
        fcp_bytes.push(TAG_DF_NAME);
        fcp_bytes.push(aid_bytes.len() as u8);
        fcp_bytes.extend(aid_bytes);
        // Update Tag '62' length at index 1
        fcp_bytes[1] = (fcp_bytes.len() - 2) as u8;
    }
    hex::encode_upper(fcp_bytes)
}

pub(crate) fn find_df(df: &DedicatedFile, id: u16) -> Option<&DedicatedFile> {
    if df.file_id == id {
        return Some(df);
    }
    for file in &df.files {
        if let SimFile::DedicatedFile(df) = file
            && let Some(found) = find_df(df, id)
        {
            return Some(found);
        }
    }
    None
}

pub(crate) fn find_ef(df: &DedicatedFile, id: u16) -> Option<&ElementaryFile> {
    for file in &df.files {
        match file {
            SimFile::ElementaryFile(ef) => {
                if ef.file_id == id {
                    return Some(ef);
                }
            }
            SimFile::DedicatedFile(df) => {
                if let Some(ef) = find_ef(df, id) {
                    return Some(ef);
                }
            }
        }
    }
    None
}

pub(crate) fn find_ef_mut(df: &mut DedicatedFile, id: u16) -> Option<&mut ElementaryFile> {
    for file in &mut df.files {
        match file {
            SimFile::ElementaryFile(ef) => {
                if ef.file_id == id {
                    return Some(ef);
                }
            }
            SimFile::DedicatedFile(df) => {
                if let Some(ef) = find_ef_mut(df, id) {
                    return Some(ef);
                }
            }
        }
    }
    None
}

fn decode_imsi(bytes: &[u8]) -> Option<String> {
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

fn decode_msisdn(bytes: &[u8]) -> Option<String> {
    // Minimum size of EF_MSISDN is 28 bytes.
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

fn format_sim_payload_data(data_hex: &str, status_word: u16) -> String {
    let combined_len = data_hex.len() + 4;
    let mut result = String::with_capacity(combined_len + 6); // 6 for len and comma
    write!(&mut result, "{},{}{:04X}", combined_len, data_hex, status_word).unwrap();
    result
}

fn format_sim_payload_status(status_word: u16) -> String {
    format_sim_payload_data("", status_word)
}

// Helper functions for state synchronization and status word parsing
fn get_status_word_from_payload(payload: &str) -> Option<u16> {
    let part2 = payload.split(',').nth(1)?;
    if part2.len() < 4 {
        return None;
    }
    let sw_str = &part2[part2.len() - 4..];
    u16::from_str_radix(sw_str, 16).ok()
}

fn is_error_sw(sw: u16) -> bool {
    let high = (sw >> 8) as u8;
    !matches!(high, 0x90 | 0x91 | 0x92 | 0x9F | 0x61 | 0x62 | 0x63)
}

fn get_channel_idx_from_open_response(payload: &str) -> Option<usize> {
    let part2 = payload.split(',').nth(1)?;
    if part2.len() < 6 {
        return None;
    }
    let channel_hex = &part2[0..2];
    usize::from_str_radix(channel_hex, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            DedicatedFile, ElementaryFile, FileSystem, PinProfile, SimFile, SimIo, SimProfile,
        },
        parser::QuotedString,
        types::FacilityLockMode,
    };

    fn create_test_fdn_profile(fdn_data: Vec<u8>) -> SimProfile {
        SimProfile {
            iccid: "89014103211118500720".to_string(),
            imsi: "310260123456789".to_string(),
            pin_profile: PinProfile { pin2: "5678".to_string(), ..Default::default() },
            sim_io: SimIo {
                file_system: FileSystem {
                    master_file: DedicatedFile {
                        file_id: 0x3F00,
                        files: vec![
                            SimFile::ElementaryFile(ElementaryFile {
                                file_id: 0x2FE2,
                                record_len: None,
                                data: hex::decode("89014103211118500720").unwrap(),
                            }),
                            SimFile::DedicatedFile(DedicatedFile {
                                file_id: 0x7F10, // DF_TELECOM
                                files: vec![SimFile::ElementaryFile(ElementaryFile {
                                    file_id: 0x6F3B, // EF_FDN
                                    record_len: Some(28),
                                    data: fdn_data,
                                })],
                            }),
                        ],
                    },
                },
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_is_fdn_allowed_disabled_by_default() {
        let mut fdn_record = vec![0xFF; 28];
        fdn_record[14] = 4; // len: 3 BCD + 1 TON
        fdn_record[15] = 0x81; // National
        fdn_record[16] = 0x21; // '1','2'
        fdn_record[17] = 0x43; // '3','4'
        fdn_record[18] = 0xF5; // '5', filler

        let profile = create_test_fdn_profile(fdn_record);
        let service = SimService::new(&profile);

        assert!(service.is_fdn_allowed(&PhoneNumber::new("98765")));
        assert!(service.is_fdn_allowed(&PhoneNumber::new("12345")));
    }

    #[test]
    fn test_is_fdn_allowed_enabled_matching() {
        let mut fdn_record = vec![0xFF; 28];
        fdn_record[14] = 4; // len
        fdn_record[15] = 0x81;
        fdn_record[16] = 0x21;
        fdn_record[17] = 0x43;
        fdn_record[18] = 0xF5; // "12345"

        let mut profile = create_test_fdn_profile(fdn_record);
        profile.pin_profile.pin2 = "5678".to_string();
        let mut service = SimService::new(&profile);

        service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"5678"))).unwrap();
        assert!(service.fdn_enabled);

        assert!(service.is_fdn_allowed(&PhoneNumber::new("12345")));
        assert!(service.is_fdn_allowed(&PhoneNumber::new("1234567")));
        assert!(!service.is_fdn_allowed(&PhoneNumber::new("1234")));
        assert!(!service.is_fdn_allowed(&PhoneNumber::new("98765")));

        // Security bypass fix verification (characters 'a' in BCD)
        let mut fdn_record_with_a = vec![0xFF; 28];
        fdn_record_with_a[14] = 3; // len: 2 bytes BCD + 1 TON
        fdn_record_with_a[15] = 0x81;
        fdn_record_with_a[16] = 0x21; // "12"
        fdn_record_with_a[17] = 0xC3; // "3a"

        let profile_a = create_test_fdn_profile(fdn_record_with_a);
        let mut service_a = SimService::new(&profile_a);
        service_a.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"5678"))).unwrap();

        assert!(!service_a.is_fdn_allowed(&PhoneNumber::new("12345")));
    }

    #[test]
    fn test_handle_set_fdn_lock_lockout() {
        let fdn_record = vec![0xFF; 28];
        let mut profile = create_test_fdn_profile(fdn_record);
        profile.pin_profile.pin2 = "5678".to_string();
        profile.pin_profile.pin2_retries = Some(3);
        let mut service = SimService::new(&profile);

        // Try invalid length PIN2 -> fails immediately, does NOT decrement retries
        let res = service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"123")));
        assert_eq!(res.err(), Some(ExecutionResult::cme_error(CmeError::IncorrectPassword)));
        assert_eq!(service.pin2_retries, 3);

        let res =
            service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"123456789")));
        assert_eq!(res.err(), Some(ExecutionResult::cme_error(CmeError::IncorrectPassword)));
        assert_eq!(service.pin2_retries, 3);

        // Try wrong PIN2 -> retries decrement
        let res = service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"0000")));
        assert!(res.is_err());
        assert_eq!(service.pin2_retries, 2);

        // Try wrong PIN2 again
        let res = service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"0000")));
        assert!(res.is_err());
        assert_eq!(service.pin2_retries, 1);

        // Try wrong PIN2 third time -> retries reach 0
        let res = service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"0000")));
        assert!(res.is_err());
        assert_eq!(service.pin2_retries, 0);

        // Try CORRECT PIN2 now that it is blocked -> should STILL fail with
        // SimPuk2Required!
        let res = service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"5678")));
        assert_eq!(res.err(), Some(ExecutionResult::cme_error(CmeError::SimPuk2Required)));

        assert!(!service.fdn_enabled);
    }
}
