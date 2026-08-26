// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, fmt::Write, iter::once};

use modem_rs_derive::CommandParser;
use tracing::{debug, info};

use crate::{
    apdu,
    config::{FileSystem, OverwritePolicy, ProfileMetadata, SimFile, SimProfile},
    constants::*,
    parser::{ApduData, PinString, QuotedString, parse_raw_data},
    types::{
        AdnRecord, CdmaRoamingPreference, CdmaSubscriptionSource, CmeError, DEFAULT_PIN,
        DEFAULT_PIN2, ExecutionResult, FacilityLockMode, Parsable, PhoneNumber, Plmn,
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
        command: apdu::Instruction,
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
const STATUS_FCP_HEX: &str = "62338202782183023F00A50C80016187010183040007DBF08A01058B062F0601020002C60C90016083010183010A83010D8102FFFF";

/// 3GPP TS 51.011 §9.3 Access Condition Levels.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AccessLevel {
    #[default]
    Always = 0x0,
    Pin1 = 0x1,
    Pin2 = 0x2,
    Never = 0xF,
}

/// 3GPP TS 51.011 §9.2.1 Access Conditions (Bytes 9–12).
///
/// Encodes 4-bit nibbles:
/// - Byte 9:  `[read: 4 bits | update: 4 bits]`
/// - Byte 10: `[increase: 4 bits | rfu: 4 bits]`
/// - Byte 11: `[rehabilitate: 4 bits | invalidate: 4 bits]`
/// - Byte 12: RFU / Administrative management (`0x00`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccessConditions {
    pub read: AccessLevel,
    pub update: AccessLevel,
    pub increase: AccessLevel,
    pub rehabilitate: AccessLevel,
    pub invalidate: AccessLevel,
}

impl AccessConditions {
    pub fn to_bytes(self) -> [u8; 4] {
        [
            ((self.read as u8) << 4) | (self.update as u8),
            (self.increase as u8) << 4,
            ((self.rehabilitate as u8) << 4) | (self.invalidate as u8),
            0x00, // Byte 12: RFU
        ]
    }
}

/// 3GPP TS 51.011 §9.2.1 Byte 13: Elementary File Status.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FileStatus {
    #[default]
    Valid = 0x00,
    Invalidated = 0x01,
}

/// 3GPP TS 51.011 §9.2.1 Byte 13: DF Characteristics (Clock Stop mode and
/// preference).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ClockStopPreference {
    #[default]
    NotAllowed = 0x00,
    NoPreference = 0x04,
    HighLevel = 0x05,
    LowLevel = 0x06,
}

/// 3GPP TS 51.011 §9.2.1 Bytes 16–22: CHV and Administrative Status for DF/MF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChvStatus {
    pub num_chvs: u8,
    pub chv1_status: u8,
    pub unblock_chv1_status: u8,
    pub chv2_status: u8,
    pub unblock_chv2_status: u8,
}

impl ChvStatus {
    pub fn to_bytes(self) -> [u8; 7] {
        [
            self.num_chvs,
            self.chv1_status,
            self.unblock_chv1_status,
            self.chv2_status,
            self.unblock_chv2_status,
            0x00, // Byte 21: RFU
            0x00, // Byte 22: RFU / Administrative data
        ]
    }
}

/// SIM File Type in 3GPP TS 51.011 §9.2.1 response header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SimFileType {
    Master = 0x01,
    Dedicated = 0x02,
    Elementary = 0x04,
}

/// Elementary File (EF) structure in 3GPP TS 51.011 §9.2.1 byte 14.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ElementaryFileStructure {
    Transparent = 0x00,
    LinearFixed = 0x01,
}

/// 3GPP TS 51.011 §9.2.1 response header for Elementary Files (EF).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementaryFileResponseHeader {
    pub file_size: u16,
    pub file_id: u16,
    pub file_type: SimFileType,
    pub access_conditions: AccessConditions,
    pub file_status: FileStatus,
    pub structure: ElementaryFileStructure,
    pub record_len: Option<u8>,
}

impl ElementaryFileResponseHeader {
    pub fn new(file_id: u16, file_size: usize, record_len: Option<usize>) -> Self {
        let (structure, r_len) = match record_len {
            Some(rlen) => (ElementaryFileStructure::LinearFixed, Some(rlen as u8)),
            None => (ElementaryFileStructure::Transparent, None),
        };
        Self {
            file_size: file_size as u16,
            file_id,
            file_type: SimFileType::Elementary,
            access_conditions: AccessConditions::default(),
            file_status: FileStatus::Valid,
            structure,
            record_len: r_len,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let following_data: &[u8] = match self.structure {
            ElementaryFileStructure::LinearFixed => {
                &[0x02, self.structure as u8, self.record_len.unwrap_or(0)]
            }
            ElementaryFileStructure::Transparent => &[0x00, self.structure as u8],
        };

        [0x00, 0x00] // Bytes 1-2: RFU
            .into_iter()
            .chain(self.file_size.to_be_bytes()) // Bytes 3-4: File size
            .chain(self.file_id.to_be_bytes()) // Bytes 5-6: File ID
            .chain(once(self.file_type as u8)) // Byte 7: File type
            .chain(once(0x00)) // Byte 8: Cyclic increase pointer / RFU
            .chain(self.access_conditions.to_bytes()) // Bytes 9-12: Access conditions
            .chain(once(self.file_status as u8)) // Byte 13: File status
            .chain(following_data.iter().copied()) // Bytes 14+: Following data & structure
            .collect()
    }
}

/// 3GPP TS 51.011 §9.2.1 response header for Dedicated Files (DF) / Master File
/// (MF).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedicatedFileResponseHeader {
    pub memory_allocated: u16,
    pub file_id: u16,
    pub file_type: SimFileType,
    pub clock_stop: ClockStopPreference,
    pub num_df_children: u8,
    pub num_ef_children: u8,
    pub chv_status: ChvStatus,
}

impl DedicatedFileResponseHeader {
    pub fn new(file_id: u16, is_mf: bool, num_df_children: usize, num_ef_children: usize) -> Self {
        Self {
            memory_allocated: 0,
            file_id,
            file_type: if is_mf { SimFileType::Master } else { SimFileType::Dedicated },
            clock_stop: ClockStopPreference::default(),
            num_df_children: num_df_children as u8,
            num_ef_children: num_ef_children as u8,
            chv_status: ChvStatus::default(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        [0x00, 0x00] // Bytes 1-2: RFU
            .into_iter()
            .chain(self.memory_allocated.to_be_bytes()) // Bytes 3-4: Memory allocated
            .chain(self.file_id.to_be_bytes()) // Bytes 5-6: File ID
            .chain(once(self.file_type as u8)) // Byte 7: File type
            .chain([0x00; 5]) // Bytes 8-12: RFU
            .chain(once(self.clock_stop as u8)) // Byte 13: DF characteristics
            .chain(once(self.num_df_children)) // Byte 14: Direct child DFs
            .chain(once(self.num_ef_children)) // Byte 15: Direct child EFs
            .chain(self.chv_status.to_bytes()) // Bytes 16-22: CHV & admin status
            .collect()
    }
}

/// Strongly-typed container for SIM file response headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimResponseHeader {
    Elementary(ElementaryFileResponseHeader),
    Dedicated(DedicatedFileResponseHeader),
}

impl SimResponseHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Elementary(ef) => ef.to_bytes(),
            Self::Dedicated(df) => df.to_bytes(),
        }
    }
}

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
    PermBlocked,
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
    provisioned: bool,
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

impl Default for SimService {
    fn default() -> Self {
        Self {
            provisioned: false,
            state: SimState::Absent,
            pin_enabled: false,
            pin1: DEFAULT_PIN.to_string(),
            puk1: DEFAULT_PUK.to_string(),
            pin1_retries: DEFAULT_PIN_RETRIES,
            puk1_retries: DEFAULT_PUK_RETRIES,
            pin2: DEFAULT_PIN2.to_string(),
            pin2_retries: DEFAULT_PIN_RETRIES,
            puk2_retries: DEFAULT_PUK_RETRIES,
            fdn_enabled: false,
            fs: FileSystem::default(),
            sms_messages: HashMap::new(),
            // Channel 0 is the basic channel and is always open by default.
            logical_channels: [true, false, false, false],
            selected_aids: [const { None }; 4],
            selected_files: [None; 4],
            response_buffer: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            cdma_subscription_source: CdmaSubscriptionSource::default(),
            cdma_roaming_preference: CdmaRoamingPreference::default(),
            adfs: Vec::new(),
            eid: None,
            atr: None,
        }
    }
}

impl SimService {
    /// Creates a new unloaded SimService.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new SimService with the given profile loaded.
    pub fn from_profile(profile: &SimProfile) -> Self {
        let mut service = Self::new();
        service.load_profile(profile);
        service
    }

    /// Returns true if a profile is currently provisioned in this SIM slot.
    pub(crate) fn is_provisioned(&self) -> bool {
        self.provisioned
    }

    /// Unprovisions and completely wipes the SIM card, restoring to default.
    fn clear_profile(&mut self) {
        *self = Self::default();
    }

    /// Loads a SIM profile configuration into the service, rebuilding the
    /// filesystem and resetting credentials and logical channels.
    pub fn load_profile(&mut self, profile: &SimProfile) {
        self.provisioned = true;
        self.state = if profile.pin_profile.puk1_retries == Some(0)
            || profile.pin_profile.state == crate::config::PinState::PermBlocked
        {
            SimState::PermBlocked
        } else {
            match profile.pin_profile.state {
                crate::config::PinState::EnabledNotVerified => SimState::PinRequired,
                crate::config::PinState::Blocked => SimState::PukRequired,
                _ => SimState::Ready,
            }
        };
        self.pin_enabled = matches!(
            profile.pin_profile.state,
            crate::config::PinState::EnabledNotVerified | crate::config::PinState::EnabledVerified
        );
        self.pin1 = if !profile.pin_profile.pin1.is_empty() {
            profile.pin_profile.pin1.clone()
        } else {
            DEFAULT_PIN.to_string()
        };
        self.puk1 = if !profile.pin_profile.puk1.is_empty() {
            profile.pin_profile.puk1.clone()
        } else {
            DEFAULT_PUK.to_string()
        };
        self.pin1_retries = profile.pin_profile.pin1_retries.unwrap_or(DEFAULT_PIN_RETRIES);
        self.puk1_retries = profile.pin_profile.puk1_retries.unwrap_or(DEFAULT_PUK_RETRIES);
        self.pin2 = if !profile.pin_profile.pin2.is_empty() {
            profile.pin_profile.pin2.clone()
        } else {
            DEFAULT_PIN2.to_string()
        };
        self.pin2_retries = profile.pin_profile.pin2_retries.unwrap_or(DEFAULT_PIN_RETRIES);
        self.puk2_retries = profile.pin_profile.puk2_retries.unwrap_or(DEFAULT_PUK_RETRIES);
        self.fdn_enabled = false;
        self.sms_messages.clear();
        self.fs = profile.sim_io.file_system.clone();
        self.logical_channels = [true, false, false, false];
        self.selected_aids = [const { None }; 4];
        self.selected_files = [None; 4];
        self.response_buffer = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        self.adfs = profile.adfs.clone();
        self.eid = profile.eid.clone();
        self.atr = profile.atr.clone();

        let iccid = if profile.iccid.is_empty() { DEFAULT_FALLBACK_ICCID } else { &profile.iccid };
        let iccid_swapped = crate::pdu::bcd::string_to_bcd(iccid);

        let imsi = if profile.imsi.is_empty() { DEFAULT_FALLBACK_IMSI } else { &profile.imsi };
        let imsi_encoded = crate::pdu::bcd::encode_imsi(imsi);

        let requested_msisdn = PhoneNumber::parse(profile.msisdn.as_bytes()).map(|(_, p)| p).ok();
        self.fs.normalize_record_lengths();
        let existing_msisdn_ef = self.fs.find_ef_in_telecom_or_usim(UiccFileId::Msisdn);
        let msisdn_record_len =
            existing_msisdn_ef.and_then(|ef| ef.record_len()).unwrap_or_else(|| {
                UiccFileId::Msisdn.default_record_len().expect("file id is record based")
            });
        let msisdn_data = requested_msisdn
            .as_ref()
            .map(|p| AdnRecord::encode_from_number(p, msisdn_record_len))
            .or_else(|| existing_msisdn_ef.map(|ef| ef.data.clone()))
            .unwrap_or_else(|| vec![0xFF; msisdn_record_len]);
        let fplmn_data = EF_FPLMN_DATA_FALLBACK.to_vec();

        self.fs.ensure_ef_present(UiccFileId::Iccid, None, iccid_swapped, OverwritePolicy::Always);
        if let Some(encoded) = imsi_encoded {
            self.fs.ensure_ef_present(UiccFileId::Imsi, None, encoded, OverwritePolicy::Always);
        }
        self.fs.ensure_ef_present_in_telecom_and_usim(
            UiccFileId::Msisdn,
            Some(msisdn_record_len),
            msisdn_data,
            OverwritePolicy::IfUninitialized,
        );

        let existing_mbdn_ef =
            self.fs.find_ef_in_telecom_or_usim(UiccFileId::MailboxDialingNumbers);
        let mbdn_record_len =
            existing_mbdn_ef.and_then(|ef| ef.record_len()).unwrap_or_else(|| {
                UiccFileId::MailboxDialingNumbers
                    .default_record_len()
                    .expect("file id is record based")
            });
        let mut mbdn_data = existing_mbdn_ef
            .map(|ef| ef.data.clone())
            .unwrap_or_else(|| vec![0xFF; DEFAULT_MBDN_RECORD_COUNT * mbdn_record_len]);
        if mbdn_data.len() < DEFAULT_MBDN_RECORD_COUNT * mbdn_record_len {
            mbdn_data.resize(DEFAULT_MBDN_RECORD_COUNT * mbdn_record_len, 0xFF);
        }

        self.fs.ensure_ef_present_in_telecom_and_usim(
            UiccFileId::MailboxDialingNumbers,
            Some(mbdn_record_len),
            mbdn_data,
            OverwritePolicy::IfUninitialized,
        );

        let existing_fdn_ef = self.fs.find_ef_in_telecom_or_usim(UiccFileId::FixedDialingNumbers);
        let fdn_record_len = existing_fdn_ef.and_then(|ef| ef.record_len()).unwrap_or_else(|| {
            UiccFileId::FixedDialingNumbers.default_record_len().expect("file id is record based")
        });
        let mut fdn_data = existing_fdn_ef
            .map(|ef| ef.data.clone())
            .unwrap_or_else(|| vec![0xFF; DEFAULT_FDN_RECORD_COUNT * fdn_record_len]);
        if fdn_data.len() < DEFAULT_FDN_RECORD_COUNT * fdn_record_len {
            fdn_data.resize(DEFAULT_FDN_RECORD_COUNT * fdn_record_len, 0xFF);
        }

        self.fs.ensure_ef_present_in_telecom_and_usim(
            UiccFileId::FixedDialingNumbers,
            Some(fdn_record_len),
            fdn_data,
            OverwritePolicy::IfUninitialized,
        );

        self.fs.ensure_ef_present(
            UiccFileId::ForbiddenPlmn,
            None,
            fplmn_data,
            OverwritePolicy::Never,
        );
    }

    /// Inserts a SIM card by applying the provided profile. Fails if a SIM is
    /// already provisioned.
    pub(crate) fn insert_sim(&mut self, profile: &SimProfile) -> bool {
        if self.is_provisioned() {
            return false;
        }
        self.load_profile(profile);
        true
    }

    /// Removes and unprovisions the SIM card, clearing the filesystem and
    /// credentials.
    pub(crate) fn remove_sim(&mut self) -> bool {
        let was_provisioned = self.is_provisioned();
        self.clear_profile();
        was_provisioned
    }

    /// Returns the home PLMN (MCC + MNC) derived from the active SIM's EF_IMSI,
    /// or `None` if the SIM is absent or has no valid IMSI.
    pub(crate) fn home_plmn(&self) -> Option<Plmn> {
        self.get_imsi().and_then(|imsi| Plmn::from_imsi(&imsi, None))
    }

    /// Returns summary metadata for the active SIM profile, or `None` if the
    /// SIM is absent.
    pub(crate) fn get_profile_metadata(&self) -> Option<ProfileMetadata> {
        if !self.is_present() {
            return None;
        }
        Some(ProfileMetadata {
            iccid: self.get_iccid().unwrap_or_default(),
            imsi: self.get_imsi().unwrap_or_default(),
            msisdn: self.get_msisdn().map(|p| p.to_string()).unwrap_or_default(),
            home_plmn: self.home_plmn(),
            eid: self.eid.clone(),
        })
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
            let target_fid = u16::from_be_bytes([apdu.data()[0], apdu.data()[1]]);
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

    pub(crate) fn get_imsi(&self) -> Option<String> {
        if !self.is_present() {
            return None;
        }
        self.fs.find_ef(UiccFileId::Imsi).and_then(|ef| decode_imsi(&ef.data))
    }

    pub(crate) fn get_iccid(&self) -> Option<String> {
        if !self.is_present() {
            return None;
        }
        self.fs.find_ef(UiccFileId::Iccid).map(|ef| crate::pdu::bcd::bcd_to_string(&ef.data))
    }

    pub(crate) fn get_msisdn(&self) -> Option<PhoneNumber> {
        let ef = self.fs.find_ef_in_telecom_or_usim(UiccFileId::Msisdn)?;
        ef.records().find_map(|r| AdnRecord::decode(r).and_then(|adn| adn.number))
    }

    pub(crate) fn set_msisdn(&mut self, msisdn: Option<&PhoneNumber>) {
        let record_len = self
            .fs
            .find_ef_in_telecom_or_usim(UiccFileId::Msisdn)
            .and_then(|ef| ef.record_len())
            .unwrap_or_else(|| {
                UiccFileId::Msisdn.default_record_len().expect("file id is record based")
            });

        let encoded = msisdn
            .map(|num| AdnRecord::encode_from_number(num, record_len))
            .unwrap_or_else(|| vec![0xFF; record_len]);

        for df_id in [UiccFileId::Telecom, UiccFileId::AdfDefault] {
            if let Some(df) = self.fs.find_df_mut(df_id)
                && let Err(e) = df.update_record(UiccFileId::Msisdn, 1, &encoded)
            {
                debug!("Failed to update MSISDN record 1 in {df_id:?}: {e:?}");
            }
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
                let fid = u16::from_be_bytes([apdu.data()[0], apdu.data()[1]]);
                let is_file_in_active_adf = if let Some(active_aid) = &self.selected_aids[idx]
                    && let Some(adf) = self.adfs.iter().find(|a| a.aid == *active_aid)
                {
                    let in_overrides = adf.files.iter().any(|f| f.id == fid);
                    let in_fs_subtree =
                        if let Some(adf_df) = self.fs.find_df(UiccFileId::AdfDefault) {
                            adf_df.find_ef(fid).is_some() || adf_df.find_df(fid).is_some()
                        } else {
                            false
                        };
                    in_overrides || in_fs_subtree
                } else {
                    false
                };
                if self.fs.find_ef(fid).is_some()
                    || self.fs.find_df(fid).is_some()
                    || is_file_in_active_adf
                    || UiccFileId::is_virtual_fallback_id(fid)
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

    pub(crate) fn get_cpin_urc(&self) -> Option<String> {
        let status = match self.state {
            SimState::Absent => return Some("+CPIN: ABSENT\r\n".to_string()),
            SimState::Ready => RequiredPin::None,
            SimState::PinRequired => RequiredPin::SimPin,
            SimState::PukRequired => RequiredPin::SimPuk,
            SimState::PermBlocked => return None,
        };
        Some(format!("{}", SimResponse::PinStatus(status)))
    }

    pub(crate) fn is_present(&self) -> bool {
        self.provisioned && self.state != SimState::Absent
    }

    pub(crate) fn set_present(&mut self, present: bool) -> bool {
        if present && !self.provisioned {
            return false;
        }
        let old_state = self.state;
        if present {
            if self.state == SimState::Absent {
                self.state = if self.puk1_retries == 0 {
                    SimState::PermBlocked
                } else if self.pin1_retries == 0 {
                    SimState::PukRequired
                } else if self.pin_enabled {
                    SimState::PinRequired
                } else {
                    SimState::Ready
                };
            }
        } else {
            self.state = SimState::Absent;
            self.logical_channels = [true, false, false, false];
            self.selected_aids = [const { None }; 4];
            self.selected_files = [None; 4];
            self.response_buffer = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        }
        self.state != old_state
    }

    // --- Public API for other services ---

    pub(crate) fn get_sms_count(&self) -> usize {
        if !self.is_present() {
            return 0;
        }
        self.sms_messages.len()
    }

    pub(crate) fn store_sms(&mut self, pdu: &[u8]) -> Option<u8> {
        if !self.is_present() {
            return None;
        }
        let index = self.sms_messages.len() as u8 + 1;
        self.sms_messages.insert(index, pdu.to_vec());
        Some(index)
    }

    pub(crate) fn read_sms(&self, index: u8) -> Result<Option<Vec<u8>>, CmeError> {
        if !self.is_present() {
            return Err(CmeError::SimNotInserted);
        }
        if let Some(pdu) = self.sms_messages.get(&index) { Ok(Some(pdu.clone())) } else { Ok(None) }
    }

    pub(crate) fn delete_sms(&mut self, index: u8) -> bool {
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
            SimState::PermBlocked => return Err(ExecutionResult::cme_error(CmeError::SimFailure)),
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
                        if self.puk1_retries == 0 {
                            self.state = SimState::PermBlocked;
                        }
                        Err(ExecutionResult::cme_error(CmeError::IncorrectPassword))
                    }
                } else {
                    Err(ExecutionResult::cme_error(CmeError::IncorrectParameters))
                }
            }
            SimState::PermBlocked => Err(ExecutionResult::cme_error(CmeError::OperationNotAllowed)),
        }
    }

    fn handle_get_imsi(&self) -> SimResult {
        if let Some(imsi) = self.get_imsi() {
            Ok(Some(SimResponse::Imsi(imsi)))
        } else {
            Err(ExecutionResult::cme_error(CmeError::NotFound))
        }
    }

    fn handle_get_iccid(&self) -> SimResult {
        if let Some(iccid) = self.get_iccid() {
            Ok(Some(SimResponse::Iccid(iccid)))
        } else {
            Err(ExecutionResult::cme_error(CmeError::NotFound))
        }
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
        file_id: impl Into<u16>,
        p1: u8,
        p2: u8,
        p3: u8,
        hex_str: &str,
    ) -> Result<(), SimResponse> {
        let file_id = file_id.into();
        let is_dual_synced = UiccFileId::try_from(file_id).is_ok_and(UiccFileId::is_dual_df_synced);

        if is_dual_synced {
            let mut updated = false;
            for df_id in [UiccFileId::Telecom, UiccFileId::AdfDefault] {
                if let Some(df) = self.fs.find_df_mut(df_id)
                    && let Some(ef) = df.find_ef_mut(file_id)
                {
                    ef.update(command, p1, p2, p3, hex_str).map_err(map_sw_to_response)?;
                    updated = true;
                }
            }
            if updated {
                return Ok(());
            }
        }

        // Update in the file system if it exists there
        if let Some(ef) = self.fs.find_ef_mut(file_id) {
            ef.update(command, p1, p2, p3, hex_str).map_err(map_sw_to_response)
        } else if UiccFileId::is_virtual_fallback_id(file_id) {
            Ok(())
        } else {
            Err(RESP_FILE_NOT_FOUND)
        }
    }

    fn read_binary_from_fs(
        &self,
        file_id: impl Into<u16>,
        p1: u8,
        p2: u8,
        p3: u8,
    ) -> Result<String, SimResponse> {
        let file_id = file_id.into();
        if let Some(ef) = self.fs.find_ef(file_id) {
            let offset = u16::from_be_bytes([p1, p2]) as usize;
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
        file_id: impl Into<u16>,
        record_num: u8,
        p2: u8,
        p3: u8,
    ) -> Result<String, SimResponse> {
        if let Ok(mode) = apdu::RecordMode::try_from(p2) {
            if !mode.is_absolute() {
                return Err(RESP_INCORRECT_PARAMS);
            }
            let file_id = file_id.into();
            if let Some(ef) = self.fs.find_ef(file_id) {
                if let Some(record) = ef.record(record_num as usize) {
                    let p3_usize = p3 as usize;
                    if p3_usize > record.len() {
                        return Err(RESP_WRONG_LENGTH);
                    }
                    let length = if p3_usize == 0 { record.len() } else { p3_usize };
                    return Ok(hex::encode_upper(&record[..length]));
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
        command: apdu::Instruction,
        file_id: u16,
        p1: u8,
        p2: u8,
        p3: u8,
        data: Option<String>,
    ) -> SimResult {
        // 1. Handle UPDATE BINARY and UPDATE RECORD
        if command.is_update() {
            let resp = if let Some(hex_str) = data {
                match self.update_sim_file(command, file_id, p1, p2, p3, &hex_str) {
                    Ok(()) => RESP_SUCCESS,
                    Err(err_resp) => err_resp,
                }
            } else {
                RESP_INCORRECT_PARAMS
            };
            return Ok(Some(resp));
        }

        // 2. Try to read from the loaded FileSystem first (for READ BINARY and SELECT)
        if command == apdu::Instruction::ReadBinary {
            match self.read_binary_from_fs(file_id, p1, p2, p3) {
                Ok(data_hex) => {
                    return Ok(Some(SimResponse::RestrictedSimAccess {
                        sw: SW_SUCCESS,
                        data: Some(data_hex),
                    }));
                }
                Err(err_resp) => return Ok(Some(err_resp)),
            }
        } else if command == apdu::Instruction::ReadRecord {
            match self.read_record_from_fs(file_id, p1, p2, p3) {
                Ok(record_hex) => {
                    return Ok(Some(SimResponse::RestrictedSimAccess {
                        sw: SW_SUCCESS,
                        data: Some(record_hex),
                    }));
                }
                Err(err_resp) => return Ok(Some(err_resp)),
            }
        } else if command == apdu::Instruction::Select && self.fs.find_df(file_id).is_some() {
            return Ok(Some(SimResponse::RestrictedSimAccess {
                sw: SW_SUCCESS,
                data: Some("6210".to_string()),
            }));
        } else if command == apdu::Instruction::Status {
            // Return FCP template for Master File (MF)
            return Ok(Some(SimResponse::RestrictedSimAccess {
                sw: SW_SUCCESS,
                data: Some(STATUS_FCP_HEX.to_string()),
            }));
        } else if command == apdu::Instruction::GetResponse {
            let header_opt = if let Some(ef) = self.fs.find_ef(file_id) {
                Some(SimResponseHeader::Elementary(ElementaryFileResponseHeader::new(
                    ef.file_id,
                    ef.size(),
                    ef.record_len(),
                )))
            } else if let Some(df) = self.fs.find_df(file_id) {
                let df_count =
                    df.files.iter().filter(|f| matches!(f, SimFile::DedicatedFile(_))).count();
                let ef_count =
                    df.files.iter().filter(|f| matches!(f, SimFile::ElementaryFile(_))).count();
                let is_mf = df.file_id == UiccFileId::MasterFile.as_u16();
                Some(SimResponseHeader::Dedicated(DedicatedFileResponseHeader::new(
                    df.file_id, is_mf, df_count, ef_count,
                )))
            } else {
                None
            };

            if let Some(header) = header_opt {
                let header_bytes = header.to_bytes();
                let p3_usize = p3 as usize;
                if p3_usize > header_bytes.len() {
                    return Ok(Some(RESP_WRONG_LENGTH));
                }
                let resp_data =
                    if p3_usize > 0 { &header_bytes[..p3_usize] } else { &header_bytes[..] };
                return Ok(Some(SimResponse::RestrictedSimAccess {
                    sw: SW_SUCCESS,
                    data: Some(hex::encode_upper(resp_data)),
                }));
            }
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
                            self.selected_files[channel_idx] =
                                Some(UiccFileId::MasterFile.as_u16());
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
                    match self.read_record_from_fs(
                        fid,
                        apdu.p1,
                        apdu.p2,
                        apdu.expected_length().unwrap_or(0),
                    ) {
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
                        apdu.data().len() as u8,
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
                        apdu.data().len() as u8,
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
                                    let fid = self.selected_files[idx]
                                        .unwrap_or(UiccFileId::AdfDefault.as_u16());
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
        if self.state == SimState::PermBlocked {
            return Err(ExecutionResult::cme_error(CmeError::OperationNotAllowed));
        }
        if self.state == SimState::PukRequired {
            return Err(ExecutionResult::cme_error(CmeError::SimPukRequired));
        }
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
            let cleaned = num_str.trim_start_matches('=').trim_matches('"');
            let phone = PhoneNumber::parse(cleaned.as_bytes()).map(|(_, p)| p).ok();
            self.set_msisdn(phone.as_ref());
        }
        Ok(None)
    }

    pub(crate) fn handle_set_facility_lock(
        &mut self,
        mode: FacilityLockMode,
        passwd: Option<QuotedString>,
    ) -> SimResult {
        if mode == FacilityLockMode::Unlock || mode == FacilityLockMode::Lock {
            if self.state == SimState::PermBlocked {
                return Err(ExecutionResult::cme_error(CmeError::OperationNotAllowed));
            }
            if self.state == SimState::PukRequired {
                return Err(ExecutionResult::cme_error(CmeError::SimPukRequired));
            }
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

        let Some(fdn_ef) = self.fs.find_ef(UiccFileId::FixedDialingNumbers) else {
            return false;
        };

        let normalized_target = number.normalized();
        if normalized_target.is_empty() {
            return false;
        }

        for record in fdn_ef.records() {
            if let Some(record) = AdnRecord::decode(record)
                && let Some(fdn_number) = record.number
            {
                let normalized_fdn = fdn_number.normalized();
                if !normalized_fdn.is_empty() && normalized_target.starts_with(normalized_fdn) {
                    return true;
                }
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

    pub(crate) fn execute<'a>(&mut self, command: &SimCommand<'a>) -> ExecutionResult {
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

fn generate_df_fcp(df_id: u16, active_aid: Option<&str>) -> String {
    let mut fcp_bytes = hex::decode(STATUS_FCP_HEX).unwrap();
    // Overwrite File ID in FCP template (Tag '83' at index 6: 83 02 3F 00)
    fcp_bytes[8] = ((df_id >> 8) & 0xFF) as u8;
    fcp_bytes[9] = (df_id & 0xFF) as u8;

    if df_id == UiccFileId::AdfDefault.as_u16()
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

fn map_sw_to_response(sw: u16) -> SimResponse {
    match sw {
        SW_INCORRECT_PARAMS => RESP_INCORRECT_PARAMS,
        SW_WRONG_LENGTH => RESP_WRONG_LENGTH,
        SW_REFERENCED_DATA_NOT_FOUND => RESP_REFERENCED_DATA_NOT_FOUND,
        _ => SimResponse::RestrictedSimAccess { sw, data: None },
    }
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
                        file_id: UiccFileId::MasterFile.as_u16(),
                        files: vec![
                            SimFile::ElementaryFile(ElementaryFile {
                                file_id: UiccFileId::Iccid.as_u16(),
                                record_len: None,
                                data: hex::decode("89014103211118500720").unwrap(),
                            }),
                            SimFile::DedicatedFile(DedicatedFile {
                                file_id: UiccFileId::Telecom.as_u16(),
                                files: vec![SimFile::ElementaryFile(ElementaryFile {
                                    file_id: UiccFileId::FixedDialingNumbers.as_u16(),
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
        let mut service = SimService::new();
        service.load_profile(&profile);

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
        let mut service = SimService::new();
        service.load_profile(&profile);

        service.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"5678"))).unwrap();
        assert!(service.fdn_enabled);

        assert!(service.is_fdn_allowed(&PhoneNumber::new("12345")));
        assert!(service.is_fdn_allowed(&PhoneNumber::new("1234567")));
        assert!(!service.is_fdn_allowed(&PhoneNumber::new("1234")));
        assert!(!service.is_fdn_allowed(&PhoneNumber::new("98765")));
        // Normalized international dialing matches domestic FDN entry
        assert!(service.is_fdn_allowed(&PhoneNumber::new("+12345")));
        assert!(!service.is_fdn_allowed(&PhoneNumber::new("")));

        // Security bypass fix verification (characters 'a' in BCD)
        let mut fdn_record_with_a = vec![0xFF; 28];
        fdn_record_with_a[14] = 3; // len: 2 bytes BCD + 1 TON
        fdn_record_with_a[15] = 0x81;
        fdn_record_with_a[16] = 0x21; // "12"
        fdn_record_with_a[17] = 0xC3; // "3a"

        let profile_a = create_test_fdn_profile(fdn_record_with_a);
        let mut service_a = SimService::new();
        service_a.load_profile(&profile_a);
        service_a.handle_set_fdn_lock(FacilityLockMode::Lock, Some(QuotedString(b"5678"))).unwrap();

        assert!(!service_a.is_fdn_allowed(&PhoneNumber::new("12345")));
    }

    #[test]
    fn test_handle_set_fdn_lock_lockout() {
        let fdn_record = vec![0xFF; 28];
        let mut profile = create_test_fdn_profile(fdn_record);
        profile.pin_profile.pin2 = "5678".to_string();
        profile.pin_profile.pin2_retries = Some(3);
        let mut service = SimService::new();
        service.load_profile(&profile);

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

    #[test]
    fn test_elementary_file_response_header_serialization() {
        let ef_header = ElementaryFileResponseHeader::new(0x6F40, 28, Some(28));
        let bytes = ef_header.to_bytes();
        assert_eq!(bytes.len(), 16);
        assert_eq!(bytes[0..2], [0x00, 0x00]);
        assert_eq!(bytes[2..4], [0x00, 28]);
        assert_eq!(bytes[4], 0x6F);
        assert_eq!(bytes[5], 0x40);
        assert_eq!(bytes[6], SimFileType::Elementary as u8);
        assert_eq!(bytes[7], 0x00);
        assert_eq!(bytes[8..12], [0x00, 0x00, 0x00, 0x00]);
        assert_eq!(bytes[12], FileStatus::Valid as u8);
        assert_eq!(bytes[13], 0x02);
        assert_eq!(bytes[14], ElementaryFileStructure::LinearFixed as u8);
        assert_eq!(bytes[15], 28);
    }

    #[test]
    fn test_transparent_elementary_file_response_header_serialization() {
        let ef_header = ElementaryFileResponseHeader::new(0x6F07, 9, None);
        let bytes = ef_header.to_bytes();
        assert_eq!(bytes.len(), 15);
        assert_eq!(bytes[0..2], [0x00, 0x00]);
        assert_eq!(bytes[2..4], [0x00, 9]);
        assert_eq!(bytes[4], 0x6F);
        assert_eq!(bytes[5], 0x07);
        assert_eq!(bytes[6], SimFileType::Elementary as u8);
        assert_eq!(bytes[7], 0x00);
        assert_eq!(bytes[8..12], [0x00, 0x00, 0x00, 0x00]);
        assert_eq!(bytes[12], FileStatus::Valid as u8);
        assert_eq!(bytes[13], 0x00);
        assert_eq!(bytes[14], ElementaryFileStructure::Transparent as u8);
    }

    #[test]
    fn test_access_conditions_nibble_packing() {
        let custom_ac = AccessConditions {
            read: AccessLevel::Pin1,
            update: AccessLevel::Pin2,
            increase: AccessLevel::Never,
            rehabilitate: AccessLevel::Always,
            invalidate: AccessLevel::Pin1,
        };
        let bytes = custom_ac.to_bytes();
        assert_eq!(bytes[0], 0x12); // read: 1, update: 2
        assert_eq!(bytes[1], 0xF0); // increase: F, rfu: 0
        assert_eq!(bytes[2], 0x01); // rehabilitate: 0, invalidate: 1
        assert_eq!(bytes[3], 0x00); // RFU: 00
    }

    #[test]
    fn test_chv_status_serialization() {
        let custom_chv = ChvStatus {
            num_chvs: 2,
            chv1_status: 0x83,
            unblock_chv1_status: 0x0A,
            chv2_status: 0x03,
            unblock_chv2_status: 0x0A,
        };
        let bytes = custom_chv.to_bytes();
        assert_eq!(bytes, [2, 0x83, 0x0A, 0x03, 0x0A, 0x00, 0x00]);
    }

    #[test]
    fn test_dedicated_file_response_header_serialization() {
        let df_header = DedicatedFileResponseHeader::new(0x7F10, false, 0, 2);
        let bytes = df_header.to_bytes();
        assert_eq!(bytes.len(), 22);
        assert_eq!(bytes[0..2], [0x00, 0x00]);
        assert_eq!(bytes[2..4], [0x00, 0x00]);
        assert_eq!(bytes[4], 0x7F);
        assert_eq!(bytes[5], 0x10);
        assert_eq!(bytes[6], SimFileType::Dedicated as u8);
        assert_eq!(bytes[7..12], [0x00; 5]);
        assert_eq!(bytes[12], ClockStopPreference::NotAllowed as u8);
        assert_eq!(bytes[13], 0);
        assert_eq!(bytes[14], 2);
        assert_eq!(bytes[15..22], [0x00; 7]);
    }

    #[test]
    fn test_find_df_mut() {
        let mut mf = DedicatedFile {
            file_id: UiccFileId::MasterFile.as_u16(),
            files: vec![SimFile::DedicatedFile(DedicatedFile {
                file_id: UiccFileId::Telecom.as_u16(),
                files: vec![],
            })],
        };
        let telecom = mf.find_df_mut(UiccFileId::Telecom);
        assert!(telecom.is_some());
        assert_eq!(telecom.unwrap().file_id, UiccFileId::Telecom.as_u16());
    }

    #[test]
    fn test_handle_sim_io_get_response_truncation() {
        let profile = SimProfile::default();
        let mut service = SimService::new();
        service.load_profile(&profile);
        let res = service
            .handle_sim_io(
                apdu::Instruction::GetResponse,
                UiccFileId::Msisdn.as_u16(),
                0,
                0,
                15,
                None,
            )
            .unwrap();
        if let Some(SimResponse::RestrictedSimAccess { sw, data }) = res {
            assert_eq!(sw, SW_SUCCESS);
            assert_eq!(data.unwrap().len(), 30); // 15 bytes = 30 hex characters
        } else {
            panic!("Expected RestrictedSimAccess");
        }
    }

    #[test]
    fn test_set_msisdn_preserves_subsequent_records() {
        let profile = SimProfile::default();
        let mut service = SimService::from_profile(&profile);
        let msisdn_len = UiccFileId::Msisdn.default_record_len().expect("file id is record based");

        // Populate EF_MSISDN in DF_TELECOM with 2 records
        if let Some(telecom) = service.fs.find_df_mut(UiccFileId::Telecom) {
            if let Some(ef) = telecom.find_ef_mut(UiccFileId::Msisdn) {
                ef.data = vec![0xAA; 2 * msisdn_len];
            }
        }

        let new_num = PhoneNumber::new("+15555215554");
        service.set_msisdn(Some(&new_num));

        let telecom = service.fs.find_df(UiccFileId::Telecom).unwrap();
        let ef = telecom.find_ef(UiccFileId::Msisdn).unwrap();
        assert_eq!(ef.record_count(), 2);
        // Record 1 was updated with encoded MSISDN
        assert_eq!(ef.record(1).unwrap(), &AdnRecord::encode_from_number(&new_num, msisdn_len));
        // Record 2 was preserved
        assert_eq!(ef.record(2).unwrap(), &[0xAA; 28]);
    }

    #[test]
    fn test_get_msisdn_falls_back_when_first_record_empty() {
        let record_len = 28;
        // Record 1: uninitialized (all 0xFF)
        let mut data = vec![0xFF; record_len];
        // Record 2: valid phone number "+15559876543"
        let second_number = PhoneNumber::new("+15559876543");
        data.extend(AdnRecord::encode_from_number(&second_number, record_len));

        let profile = SimProfile {
            sim_io: SimIo {
                file_system: FileSystem {
                    master_file: DedicatedFile {
                        file_id: UiccFileId::MasterFile.as_u16(),
                        files: vec![SimFile::DedicatedFile(DedicatedFile {
                            file_id: UiccFileId::Telecom.as_u16(),
                            files: vec![SimFile::ElementaryFile(ElementaryFile {
                                file_id: UiccFileId::Msisdn.as_u16(),
                                record_len: Some(record_len),
                                data,
                            })],
                        })],
                    },
                },
            },
            ..Default::default()
        };
        let service = SimService::from_profile(&profile);
        assert_eq!(service.get_msisdn(), Some(second_number));
    }

    #[test]
    fn test_sim_lifecycle_hot_swap_and_removal() {
        let default_prof =
            crate::profiles::get_builtin_profile(crate::profiles::SIM_TYPE_DEFAULT).unwrap();
        let mut service = SimService::new();
        service.load_profile(&default_prof);

        assert!(service.is_present());
        assert_eq!(service.get_imsi().as_deref(), Some("310260000000000"));
        assert_eq!(service.get_cpin_urc(), Some("+CPIN: READY\r\n".to_string()));

        let meta = service.get_profile_metadata().unwrap();
        assert_eq!(meta.imsi, "310260000000000");
        assert_eq!(meta.home_plmn.as_ref().map(Plmn::as_str), Some("310260"));

        // Store an SMS to verify it gets cleared on removal
        service.store_sms(&[1, 2, 3]);
        assert_eq!(service.get_sms_count(), 1);

        // Test ejection (set_present(false)) vs removal (remove_sim)
        assert!(service.set_present(false));
        assert!(!service.is_present());
        assert!(service.is_provisioned());
        assert_eq!(service.get_cpin_urc(), Some("+CPIN: ABSENT\r\n".to_string()));

        // Re-inserting the ejected card restores presence
        assert!(service.set_present(true));
        assert!(service.is_present());
        assert!(service.is_provisioned());
        assert_eq!(service.get_cpin_urc(), Some("+CPIN: READY\r\n".to_string()));

        // Remove SIM (unprovisions and completely clears the card)
        assert!(service.remove_sim());
        assert!(!service.is_present());
        assert!(!service.is_provisioned());
        assert_eq!(service.get_cpin_urc(), Some("+CPIN: ABSENT\r\n".to_string()));

        // Attempting to present an unprovisioned card fails
        assert!(!service.set_present(true));

        // When SIM is unprovisioned, queries return None and SMS is empty
        assert_eq!(service.get_profile_metadata(), None);
        assert_eq!(service.get_imsi(), None);
        assert_eq!(service.get_iccid(), None);
        assert_eq!(service.home_plmn(), None);
        assert_eq!(service.get_sms_count(), 0);

        // Insert Tel Alaska profile into empty slot
        let alaska_prof =
            crate::profiles::get_builtin_profile(crate::profiles::SIM_TYPE_TEL_ALASKA).unwrap();
        assert!(service.insert_sim(&alaska_prof));
        assert!(service.is_present());
        assert!(service.is_provisioned());
        assert!(!service.insert_sim(&alaska_prof));
        assert_eq!(service.get_imsi().as_deref(), Some("311740123456789"));
        assert_eq!(service.get_iccid().as_deref(), Some("89860318640220133897"));
        assert_eq!(service.get_cpin_urc(), Some("+CPIN: READY\r\n".to_string()));

        let alaska_meta = service.get_profile_metadata().unwrap();
        assert_eq!(alaska_meta.imsi, "311740123456789");
        assert_eq!(alaska_meta.home_plmn.as_ref().map(Plmn::as_str), Some("311740"));
    }

    #[test]
    fn test_load_profile_resets_logical_channels() {
        let default_prof =
            crate::profiles::get_builtin_profile(crate::profiles::SIM_TYPE_DEFAULT).unwrap();
        let mut service = SimService::new();
        service.load_profile(&default_prof);

        // Open logical channel 1
        let res = service.handle_open_logical_channel(b"").unwrap();
        assert_eq!(res, Some(SimResponse::OpenLogicalChannel(1)));
        assert!(service.logical_channels[1]);

        // Load new profile
        let cts_prof = crate::profiles::get_builtin_profile(crate::profiles::SIM_TYPE_CTS).unwrap();
        service.load_profile(&cts_prof);

        // Logical channel 1 should be closed, channel 0 open
        assert!(service.logical_channels[0]);
        assert!(!service.logical_channels[1]);
        assert_eq!(service.selected_aids[1], None);
        assert_eq!(service.selected_files[1], None);
    }

    #[test]
    fn test_load_profile_clears_sms_messages() {
        let default_prof =
            crate::profiles::get_builtin_profile(crate::profiles::SIM_TYPE_DEFAULT).unwrap();
        let mut service = SimService::new();
        service.load_profile(&default_prof);

        // Store an SMS message on the SIM card
        let dummy_pdu = [0x00, 0x01, 0x02, 0x03];
        let index = service.store_sms(&dummy_pdu);
        assert_eq!(index, Some(1));
        assert_eq!(service.get_sms_count(), 1);

        // Switching or reloading profile should clear stored SMS records
        let alaska_prof =
            crate::profiles::get_builtin_profile(crate::profiles::SIM_TYPE_TEL_ALASKA).unwrap();
        service.load_profile(&alaska_prof);
        assert_eq!(service.get_sms_count(), 0);
    }

    #[test]
    fn test_set_present_false_resets_channels_and_buffers() {
        let default_prof =
            crate::profiles::get_builtin_profile(crate::profiles::SIM_TYPE_DEFAULT).unwrap();
        let mut service = SimService::new();
        service.load_profile(&default_prof);

        // Open logical channel 1 and buffer response data
        let res = service.handle_open_logical_channel(b"").unwrap();
        assert_eq!(res, Some(SimResponse::OpenLogicalChannel(1)));
        assert!(service.logical_channels[1]);
        service.response_buffer[1] = vec![0x12, 0x34];

        // Toggling SIM presence to false must tear down logical channels, reset
        // selected files, and clear buffers
        assert!(service.set_present(false));
        assert!(!service.is_present());
        assert_eq!(service.logical_channels, [true, false, false, false]);
        assert_eq!(service.selected_aids, [const { None }; 4]);
        assert_eq!(service.selected_files, [None; 4]);
        assert!(service.response_buffer[1].is_empty());
    }
}
