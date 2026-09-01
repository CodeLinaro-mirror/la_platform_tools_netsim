// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str,
    time::Duration,
};

use netsim_model::{Call, Quirks, RegistrationStatus};
use nom::IResult;

use crate::{
    call_service::CallResponse,
    constants::{ADN_CAPABILITY_EXT_BYTES, ADN_DIALING_NUMBER_LEN, ADN_FOOTER_LEN},
    data_service::DataResponse,
    misc_service::MiscResponse,
    network_service::NetworkResponse,
    parser::QuotedString,
    sim_service::SimResponse,
    sms_service::SmsResponse,
    stk_service::StkResponse,
    sup_service::SupResponse,
};

pub trait Parsable<'a>: Sized {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self>;
}

impl Parsable<'_> for u8 {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::combinator::map_res(
            nom::combinator::map_res(nom::character::complete::digit1, str::from_utf8),
            |s: &str| s.parse::<u8>(),
        )(input)
    }
}

impl Parsable<'_> for u16 {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::combinator::map_res(
            nom::combinator::map_res(nom::character::complete::digit1, str::from_utf8),
            |s: &str| s.parse::<u16>(),
        )(input)
    }
}

impl Parsable<'_> for u32 {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::combinator::map_res(
            nom::combinator::map_res(nom::character::complete::digit1, str::from_utf8),
            |s: &str| s.parse::<u32>(),
        )(input)
    }
}

pub const AT_OK: &[u8] = b"OK\r\n";
pub const AT_ERROR: &[u8] = b"ERROR\r\n";

pub const DEFAULT_PIN: &str = "1234";
pub const DEFAULT_PIN2: &str = "5678";

pub const DEFAULT_GATEWAY: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 2);
pub const DEFAULT_DNS: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 3);
pub const DEFAULT_IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 15);

// Aligned with libslirp-rs and emulator networking defaults.
pub const DEFAULT_IPV6_GATEWAY: Ipv6Addr = Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 2);
pub const DEFAULT_IPV6_DNS: Ipv6Addr = Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 3);
pub const DEFAULT_IPV6_ADDR: Ipv6Addr = Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15);
pub const DEFAULT_IPV6_PREFIX: u32 = 64;

// A unique identifier for a modem instance.
pub type ModemId = u32;

// Custom error type for the library.
use std::fmt;

#[derive(Debug, Clone)]
pub enum ModemError {
    DuplicateModemId(ModemId),
    NotFound,
    InvalidConfig(String),
}

impl std::error::Error for ModemError {}

impl fmt::Display for ModemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModemError::DuplicateModemId(id) => write!(f, "Duplicate modem ID: {id}"),
            ModemError::NotFound => write!(f, "Modem network not found"),
            ModemError::InvalidConfig(msg) => write!(f, "Invalid configuration: {msg}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhoneNumber(String);

impl PhoneNumber {
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn normalized(&self) -> &str {
        self.0.strip_prefix('+').unwrap_or(&self.0)
    }

    pub fn is_international(&self) -> bool {
        self.0.starts_with('+')
    }

    pub fn toa(&self) -> TypeOfAddress {
        TypeOfAddress::from_number(&self.0)
    }

    pub fn is_gprs_dial(&self) -> bool {
        self.0.starts_with("*99")
            && self.0.ends_with('#')
            && self.0.as_bytes().get(3).is_some_and(|&c| c == b'*' || c == b'#')
    }
}

/// Abbreviated Dialling Number (ADN) record per 3GPP TS 31.102 §4.4.2.3 and
/// TS 51.011 §10.5.1.
///
/// Shared linear-fixed record structure for:
/// - EF_MSISDN (0x6F40)
/// - EF_MBDN (0x6FC7)
/// - EF_FDN (0x6F3B)
/// - EF_ADN (0x6F3A / 0x4F3A)
/// - EF_SDN (0x6F49)
/// - EF_BDN (0x6F4D)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdnRecord {
    pub alpha_tag: Option<String>,
    pub number: Option<PhoneNumber>,
}

impl AdnRecord {
    #[cfg(test)]
    pub fn new(alpha_tag: Option<impl Into<String>>, number: Option<PhoneNumber>) -> Self {
        Self { alpha_tag: alpha_tag.map(Into::into).filter(|s| !s.is_empty()), number }
    }

    pub fn from_number(number: &PhoneNumber) -> Self {
        Self { alpha_tag: None, number: Some(number.clone()) }
    }

    pub fn encode_from_number(number: &PhoneNumber, record_len: usize) -> Vec<u8> {
        Self::from_number(number).encode(record_len)
    }

    /// Decodes an ADN linear-fixed record slice (length >= 14 bytes).
    ///
    /// Returns None if the record is empty (all 0xFF) or malformed (< 14
    /// bytes).
    pub fn decode(record: &[u8]) -> Option<Self> {
        if record.len() < ADN_FOOTER_LEN {
            return None;
        }
        if record.iter().all(|&b| b == 0xFF) {
            return None;
        }

        let alpha_len = record.len() - ADN_FOOTER_LEN;
        let alpha_bytes = &record[..alpha_len];
        let trimmed_alpha = match alpha_bytes.iter().rposition(|&b| b != 0xFF) {
            Some(last) => &alpha_bytes[..=last],
            None => &[],
        };
        let alpha_tag = if !trimmed_alpha.is_empty() {
            Some(String::from_utf8_lossy(trimmed_alpha).into_owned())
        } else {
            None
        };

        let len_byte = record[alpha_len];
        if len_byte <= 1 || len_byte == 0xFF {
            return Some(Self { alpha_tag, number: None });
        }

        let bcd_content_len = len_byte as usize;
        if bcd_content_len > 1 + ADN_DIALING_NUMBER_LEN {
            return Some(Self { alpha_tag, number: None });
        }

        let ton_npi = record[alpha_len + 1];
        let is_international = (ton_npi & 0xF0) == 0x90 || (ton_npi & 0x70) == 0x10;

        let bcd_digits_len = bcd_content_len - 1;
        let bcd_bytes = &record[alpha_len + 2..alpha_len + 2 + bcd_digits_len];
        let mut digits = crate::pdu::bcd::bcd_to_string(bcd_bytes);

        if is_international && !digits.is_empty() && !digits.starts_with('+') {
            digits.insert(0, '+');
        }

        let number = match PhoneNumber::parse(digits.as_bytes()) {
            Ok(([], p)) => Some(p),
            _ => None,
        };
        Some(Self { alpha_tag, number })
    }

    /// Encodes this record into a linear-fixed ADN record of `record_len`
    /// bytes.
    pub fn encode(&self, record_len: usize) -> Vec<u8> {
        if record_len < ADN_FOOTER_LEN {
            return vec![0xFF; record_len];
        }
        let alpha_len = record_len - ADN_FOOTER_LEN;
        let mut out = vec![0xFF; record_len];

        // 1. Encode Alpha Identifier
        if let Some(ref tag) = self.alpha_tag {
            let tag_bytes = tag.as_bytes();
            let copy_len = tag_bytes.len().min(alpha_len);
            out[..copy_len].copy_from_slice(&tag_bytes[..copy_len]);
        }

        // 2. Encode Dialing Number
        if let Some(ref phone) = self.number {
            let clean_digits: String = phone
                .as_str()
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '*' || *c == '#')
                .collect();
            if !clean_digits.is_empty() {
                let ton_npi = if phone.as_str().starts_with('+')
                    || (clean_digits.len() == 11 && clean_digits.starts_with('1'))
                {
                    TypeOfAddress::International
                } else {
                    phone.toa()
                };

                let bcd = crate::pdu::bcd::string_to_bcd(&clean_digits);
                let dialing_copy = bcd.len().min(ADN_DIALING_NUMBER_LEN);
                let bcd_len = (1 + dialing_copy) as u8;
                out[alpha_len] = bcd_len;
                out[alpha_len + 1] = ton_npi.as_u8();
                out[alpha_len + 2..alpha_len + 2 + dialing_copy]
                    .copy_from_slice(&bcd[..dialing_copy]);
            }
        }

        // 3. Capability Configuration & Extension Record IDs (bytes 13 and 14 of
        //    footer)
        out[record_len - 2..record_len].copy_from_slice(&ADN_CAPABILITY_EXT_BYTES);

        out
    }
}

impl fmt::Display for PhoneNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of Address (TON/NPI) as defined in 3GPP TS 24.008 / TS 23.040 Table
/// 9.1.2.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeOfAddress {
    /// National / Unknown numbering plan (0x81 = 129).
    National = 129,
    /// International numbering plan with E.164 (0x91 = 145).
    International = 145,
}

impl TypeOfAddress {
    pub fn from_number(number: &str) -> Self {
        if number.starts_with('+') { Self::International } else { Self::National }
    }

    pub const fn is_international(self) -> bool {
        matches!(self, Self::International)
    }

    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for TypeOfAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl From<TypeOfAddress> for u8 {
    fn from(toa: TypeOfAddress) -> Self {
        toa as u8
    }
}

impl<'a> Parsable<'a> for PhoneNumber {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        use nom::{
            bytes::complete::{tag, take_while1},
            combinator::{opt, recognize},
            sequence::pair,
        };

        // ONLY allow clean number characters (digits, *, #, and optional leading +)
        let (remaining, digits) = recognize(pair(
            opt(tag(b"+")),
            take_while1(|c: u8| matches!(c, b'0'..=b'9' | b'*' | b'#')),
        ))(input)?;

        let s = std::str::from_utf8(digits).map_err(|_e| {
            nom::Err::Error(nom::error::Error::new(remaining, nom::error::ErrorKind::MapRes))
        })?;

        Ok((remaining, PhoneNumber(s.to_string())))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialString(String);

impl DialString {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let s = std::str::from_utf8(bytes).ok()?.trim();
        // Allow all standard dial characters and modifiers
        let is_valid = !s.is_empty()
            && s.chars().all(|c| {
                matches!(c, '0'..='9' | '*' | '#' | '+' | ',' | 'W' | 'w' | 'i' | 'I' | '@')
            });
        if !is_valid {
            return None;
        }
        Some(Self(s.to_string()))
    }

    pub fn is_emergency(&self) -> bool {
        self.0.contains('@') || self.clean_number().as_ref().map(|n| n.as_str()) == Some("911")
    }

    pub fn clir(&self, default: ClirMode) -> ClirMode {
        let at_pos = self.0.find('@');
        if let Some(pos) = at_pos {
            let parts = &self.0[pos + 1..];
            if let Some(second) = parts.split(',').nth(1) {
                let clir_part = second.trim_start_matches('#');
                if clir_part == "i" {
                    return ClirMode::Suppression;
                } else if clir_part == "I" {
                    return ClirMode::Invocation;
                }
            }
        }

        // Find CLIR suffix in the number part (before first pause/wait)
        let raw_num = if let Some(pos) = at_pos { &self.0[..pos] } else { &self.0 };
        let num_part = raw_num.split([',', 'W', 'w']).next().unwrap_or("");
        if num_part.ends_with('i') {
            ClirMode::Suppression
        } else if num_part.ends_with('I') {
            ClirMode::Invocation
        } else {
            default
        }
    }

    pub fn clean_number(&self) -> Option<PhoneNumber> {
        let raw_num = if let Some(pos) = self.0.find('@') { &self.0[..pos] } else { &self.0 };

        // Truncate at first pause/wait modifier
        let num_part = raw_num.split([',', 'W', 'w']).next().unwrap_or("");

        // Strip CLIR suffixes
        let mut clean = num_part;
        if clean.ends_with('i') || clean.ends_with('I') {
            clean = &clean[..clean.len() - 1];
        }

        // Validate clean number format strictly
        let is_valid = !clean.is_empty()
            && clean.bytes().enumerate().all(|(i, b)| match b {
                b'+' => i == 0,
                b'0'..=b'9' | b'*' | b'#' => true,
                _ => false,
            });
        if !is_valid {
            return None;
        }

        Some(PhoneNumber(clean.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialArgs {
    pub number: PhoneNumber,
    pub clir: ClirMode,
    pub is_emergency: bool,
}

impl<'a> Parsable<'a> for DialArgs {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, content) = crate::parser::parse_until_semicolon(input)?;
        let dial_str = DialString::parse(content).ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))
        })?;
        let number = dial_str.clean_number().ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
        })?;
        let clir = dial_str.clir(ClirMode::SubscriptionDefault);
        let is_emergency = dial_str.is_emergency();
        Ok((input, DialArgs { number, clir, is_emergency }))
    }
}

/// 3GPP TS 27.005 §4.4: New message acknowledgement (<n> parameter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmsAck {
    /// Message routing to TE is acknowledged (0 or 1 per TS 27.005 §4.4).
    Success,
    /// Message routing to TE is not acknowledged / rejected (2 per TS 27.005
    /// §4.4).
    Failure,
}

impl SmsAck {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}

impl<'a> Parsable<'a> for SmsAck {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 | 1 => Ok((input, Self::Success)),
            2 => Ok((input, Self::Failure)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

// Actions that a command can request to be executed by the
// CellularNetworkSimulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    InitiateCall(DialArgs),
    InitiateRemoteCall(PhoneNumber),
    InitiateEmergencyCall,
    AnswerCall(ModemId),
    HangupCall { initiator: ModemId, target_peer: ModemId },
    HoldCall { holder: ModemId, target: ModemId },
    ResumeCall { resumer: ModemId, target: ModemId },
    ReceiveSms { to: Option<String>, pdu: Vec<u8>, status_report: Option<Vec<u8>> },
    ReceiveTextSms { to: String, text: String },
    AcknowledgeIncomingSms { ack: SmsAck },
    None,
}

// Callbacks Removed in favor of Sink and NetworkEvent

use std::sync::Arc;

use bytes::Bytes;

/// A sink for sending packets back to the modem client.
#[derive(Clone)]
pub struct ModemSink {
    sender: Arc<dyn Fn(Bytes) -> Result<(), String> + Send + Sync>,
}

impl ModemSink {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Bytes) -> Result<(), String> + Send + Sync + 'static,
    {
        Self { sender: Arc::new(f) }
    }

    pub fn send(&self, packet: Bytes) -> Result<(), String> {
        (self.sender)(packet)
    }
}

impl fmt::Debug for ModemSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModemSink").finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CmeeMode {
    #[default]
    Disable = 0, // Returns "ERROR"
    Numeric = 1, // Returns "+CME ERROR: <code>"
    Verbose = 2, // Returns "+CME ERROR: <verbose string>"
}

impl<'a> Parsable<'a> for CmeeMode {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Disable)),
            1 => Ok((input, Self::Numeric)),
            2 => Ok((input, Self::Verbose)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RegistrationUnsolicitedMode {
    #[default]
    Disable = 0,
    Enable = 1,
    EnableWithLocation = 2,
}

impl<'a> Parsable<'a> for RegistrationUnsolicitedMode {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Disable)),
            1 => Ok((input, Self::Enable)),
            2 => Ok((input, Self::EnableWithLocation)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RadioPowerLevel {
    Minimum = 0,
    #[default]
    Full = 1,
    DisableRf = 4,
}

impl<'a> Parsable<'a> for RadioPowerLevel {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Minimum)),
            1 => Ok((input, Self::Full)),
            4 => Ok((input, Self::DisableRf)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

/// Standardized 3GPP TS 27.007 Section 9.2 Mobile Equipment Error Codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmeError {
    PhoneFailure,
    OperationNotAllowed,
    OperationNotSupported,
    SimNotInserted,
    SimPinRequired,
    SimPukRequired,
    SimFailure,
    SimBusy,
    IncorrectPassword,
    SimPin2Required,
    SimPuk2Required,
    MemoryFull,
    InvalidIndex,
    NotFound,
    MemoryFailure,
    TextStringTooLong,
    InvalidCharacters,
    NoNetworkService,
    NoResources,
    IncorrectParameters,
    FixedDialNumberOnlyAllowed,
    Custom(u32, &'static str),
}

impl CmeError {
    pub fn code(&self) -> u32 {
        match *self {
            Self::PhoneFailure => 0,
            Self::OperationNotAllowed => 3,
            Self::OperationNotSupported => 4,
            Self::SimNotInserted => 10,
            Self::SimPinRequired => 11,
            Self::SimPukRequired => 12,
            Self::SimFailure => 13,
            Self::SimBusy => 14,
            Self::IncorrectPassword => 16,
            Self::SimPin2Required => 17,
            Self::SimPuk2Required => 18,
            Self::MemoryFull => 20,
            Self::InvalidIndex => 21,
            Self::NotFound => 22,
            Self::MemoryFailure => 23,
            Self::TextStringTooLong => 24,
            Self::InvalidCharacters => 25,
            Self::NoNetworkService => 30,
            Self::NoResources => 142,
            Self::IncorrectParameters => 50,
            Self::FixedDialNumberOnlyAllowed => 56,
            Self::Custom(c, _) => c,
        }
    }

    pub fn verbose_str(&self) -> &'static str {
        match *self {
            Self::PhoneFailure => "phone failure",
            Self::OperationNotAllowed => "operation not allowed",
            Self::OperationNotSupported => "operation not supported",
            Self::SimNotInserted => "SIM not inserted",
            Self::SimPinRequired => "SIM PIN required",
            Self::SimPukRequired => "SIM PUK required",
            Self::SimFailure => "SIM failure",
            Self::SimBusy => "SIM busy",
            Self::IncorrectPassword => "incorrect password",
            Self::SimPin2Required => "SIM PIN2 required",
            Self::SimPuk2Required => "SIM PUK2 required",
            Self::MemoryFull => "memory full",
            Self::InvalidIndex => "invalid index",
            Self::NotFound => "not found",
            Self::MemoryFailure => "memory failure",
            Self::TextStringTooLong => "text string too long",
            Self::InvalidCharacters => "invalid characters in text string",
            Self::NoNetworkService => "no network service",
            Self::NoResources => "no resources",
            Self::IncorrectParameters => "incorrect parameters",
            Self::FixedDialNumberOnlyAllowed => "fixed dialing number only allowed",
            Self::Custom(_, msg) => msg,
        }
    }

    pub fn format_response(&self, mode: CmeeMode) -> String {
        match mode {
            CmeeMode::Disable => "ERROR\r\n".to_string(),
            CmeeMode::Numeric => format!("+CME ERROR: {}\r\n", self.code()),
            CmeeMode::Verbose => format!("+CME ERROR: {}\r\n", self.verbose_str()),
        }
    }
}

/// Combined structured response enum across all modem-rs services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Sim(SimResponse),
    Call(CallResponse),
    Sms(SmsResponse),
    Network(NetworkResponse),
    Data(DataResponse),
    Misc(MiscResponse),
    Sup(SupResponse),
    Stk(StkResponse),
    Ok,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sim(resp) => write!(f, "{resp}"),
            Self::Call(resp) => write!(f, "{resp}"),
            Self::Sms(resp) => write!(f, "{resp}"),
            Self::Network(resp) => write!(f, "{resp}"),
            Self::Data(resp) => write!(f, "{resp}"),
            Self::Misc(resp) => write!(f, "{resp}"),
            Self::Sup(resp) => write!(f, "{resp}"),
            Self::Stk(resp) => write!(f, "{resp}"),
            Self::Ok => write!(f, "OK\r\n"),
        }
    }
}

impl From<SimResponse> for Response {
    fn from(r: SimResponse) -> Self {
        Self::Sim(r)
    }
}

impl From<CallResponse> for Response {
    fn from(r: CallResponse) -> Self {
        Self::Call(r)
    }
}

impl From<SmsResponse> for Response {
    fn from(r: SmsResponse) -> Self {
        Self::Sms(r)
    }
}

impl From<NetworkResponse> for Response {
    fn from(r: NetworkResponse) -> Self {
        Self::Network(r)
    }
}

impl From<DataResponse> for Response {
    fn from(r: DataResponse) -> Self {
        Self::Data(r)
    }
}

impl From<MiscResponse> for Response {
    fn from(r: MiscResponse) -> Self {
        Self::Misc(r)
    }
}

impl From<SupResponse> for Response {
    fn from(r: SupResponse) -> Self {
        Self::Sup(r)
    }
}

impl From<StkResponse> for Response {
    fn from(r: StkResponse) -> Self {
        Self::Stk(r)
    }
}

/// Contains all the results of a successfully executed command.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HandledCommand {
    /// The immediate responses to send back to the client.
    pub responses: Vec<Response>,
    /// An optional follow-up action for the CellularNetworkSimulator to
    /// perform.
    pub actions: Vec<CommandAction>,
}

impl HandledCommand {
    /// Creates a result with a simple "OK" response and no follow-up action.
    pub fn ok() -> Self {
        Self { responses: vec![Response::Ok], actions: vec![] }
    }

    /// Creates a result with a simple "OK" response AND a follow-up action.
    pub fn ok_with_actions(actions: Vec<CommandAction>) -> Self {
        Self { responses: vec![Response::Ok], actions }
    }
}

/// Represents the outcome of a command execution from the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// The command was successfully handled, yielding responses and/or action.
    Success(HandledCommand),

    /// The command failed, optionally with a structured Mobile Equipment error
    /// and/or URCs emitted prior to failure.
    Error { cme: Option<CmeError>, urcs: Vec<Response> },

    /// This command has not been refactored yet and should be handled by the
    /// legacy system.
    Unhandled,
}

impl ExecutionResult {
    pub fn ok() -> Self {
        Self::Success(HandledCommand::ok())
    }

    pub fn ok_with_actions(actions: Vec<CommandAction>) -> Self {
        Self::Success(HandledCommand::ok_with_actions(actions))
    }

    pub fn error() -> Self {
        Self::Error { cme: None, urcs: Vec::new() }
    }

    pub fn cme_error(cme: CmeError) -> Self {
        Self::Error { cme: Some(cme), urcs: Vec::new() }
    }
}

impl<T: Into<Response>> From<Option<T>> for ExecutionResult {
    fn from(opt: Option<T>) -> Self {
        match opt.map(Into::into) {
            Some(
                resp @ (Response::Call(CallResponse::Ring) | Response::Data(DataResponse::Connect)),
            ) => Self::Success(HandledCommand { responses: vec![resp], actions: vec![] }),
            Some(Response::Call(CallResponse::Empty)) => Self::Success(HandledCommand::default()),
            Some(Response::Call(CallResponse::WithActions(actions))) => {
                Self::Success(HandledCommand::ok_with_actions(actions))
            }
            Some(resp) => Self::Success(HandledCommand {
                responses: vec![resp, Response::Ok],
                actions: vec![],
            }),
            None => Self::ok(),
        }
    }
}

impl<T: Into<ExecutionResult>> From<Result<T, ExecutionResult>> for ExecutionResult {
    fn from(res: Result<T, ExecutionResult>) -> Self {
        match res {
            Ok(val) => val.into(),
            Err(err) => err,
        }
    }
}

impl From<CmeError> for ExecutionResult {
    fn from(err: CmeError) -> Self {
        Self::cme_error(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEvent {
    SinkError(u32),
    TimerRequest { chip_id: u32, duration: Duration },
}

#[derive(Debug, Clone, Default)]
pub struct ModemInfo {
    pub id: u32,
    pub calls: Vec<Call>,
    pub ringing: bool,
    pub sms_count: u32,
    pub quirks: Quirks,
    pub rssi: u32,
    pub ber: u32,
    pub voice_registration: RegistrationStatus,
    pub data_registration: RegistrationStatus,
}

/// Represents the signal strength parameters for all supported tech layout (22
/// fields). Default values are initialized to standard "unknown" values (99 for
/// RSSI, i32::MAX for others).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalStrength {
    pub gsm_rssi: i32,
    pub gsm_ber: i32,
    pub cdma_dbm: i32,
    pub cdma_ecio: i32,
    pub evdo_dbm: i32,
    pub evdo_ecio: i32,
    pub evdo_snr: i32,
    pub lte_rssi: i32,
    pub lte_rsrp: i32,
    pub lte_rsrq: i32,
    pub lte_rssnr: i32,
    pub lte_cqi: i32,
    pub lte_ta: i32,
    pub tdscdma_rscp: i32,
    pub wcdma_rssi: i32,
    pub wcdma_ber: i32,
    pub nr_ss_rsrp: i32,
    pub nr_ss_rsrq: i32,
    pub nr_ss_sinr: i32,
    pub nr_csi_rsrp: i32,
    pub nr_csi_rsrq: i32,
    pub nr_csi_sinr: i32,
}

impl Default for SignalStrength {
    fn default() -> Self {
        let max = i32::MAX;
        let unknown = crate::constants::CSQ_SIGNAL_UNKNOWN as i32;
        Self {
            gsm_rssi: unknown,
            gsm_ber: unknown,
            cdma_dbm: max,
            cdma_ecio: max,
            evdo_dbm: max,
            evdo_ecio: max,
            evdo_snr: max,
            lte_rssi: max,
            lte_rsrp: max,
            lte_rsrq: max,
            lte_rssnr: max,
            lte_cqi: max,
            lte_ta: max,
            tdscdma_rscp: max,
            wcdma_rssi: max,
            wcdma_ber: max,
            nr_ss_rsrp: max,
            nr_ss_rsrq: max,
            nr_ss_sinr: max,
            nr_csi_rsrp: max,
            nr_csi_rsrq: max,
            nr_csi_sinr: max,
        }
    }
}

impl SignalStrength {
    pub fn to_csq_response(&self) -> String {
        format!(
            "+CSQ: {},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\r\n",
            self.gsm_rssi,
            self.gsm_ber,
            self.cdma_dbm,
            self.cdma_ecio,
            self.evdo_dbm,
            self.evdo_ecio,
            self.evdo_snr,
            self.lte_rssi,
            self.lte_rsrp,
            self.lte_rsrq,
            self.lte_rssnr,
            self.lte_cqi,
            self.lte_ta,
            self.tdscdma_rscp,
            self.wcdma_rssi,
            self.wcdma_ber,
            self.nr_ss_rsrp,
            self.nr_ss_rsrq,
            self.nr_ss_sinr,
            self.nr_csi_rsrp,
            self.nr_csi_rsrq,
            self.nr_csi_sinr
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CopsMode {
    #[default]
    Automatic = 0,
    Manual = 1,
    Deregister = 2,
    SetFormatOnly = 3,
    ManualAutomatic = 4,
}

impl<'a> Parsable<'a> for CopsMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Automatic)),
            1 => Ok((input, Self::Manual)),
            2 => Ok((input, Self::Deregister)),
            3 => Ok((input, Self::SetFormatOnly)),
            4 => Ok((input, Self::ManualAutomatic)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CopsFormat {
    LongAlphanumeric = 0,
    ShortAlphanumeric = 1,
    #[default]
    Numeric = 2,
}

impl<'a> Parsable<'a> for CopsFormat {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::LongAlphanumeric)),
            1 => Ok((input, Self::ShortAlphanumeric)),
            2 => Ok((input, Self::Numeric)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OperatorStatus {
    Unknown = 0,
    Available = 1,
    Current = 2,
    Forbidden = 3,
}

impl std::fmt::Display for OperatorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

/// AT+CHLD Call Hold operations (3GPP TS 27.007 Section 7.22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CallHoldAction {
    /// AT+CHLD=0: Releases all held calls or rejects a waiting call.
    ReleaseHeld = 0,
    /// AT+CHLD=1: Releases all active calls and accepts held/waiting call.
    ReleaseAndAccept = 1,
    /// AT+CHLD=2: Places active calls on hold and accepts held/waiting call.
    HoldAndAccept = 2,
    /// AT+CHLD=3: Adds a held call to the active conversation (Conference
    /// call).
    Conference = 3,
    /// AT+CHLD=4: Explicit Call Transfer (ECT).
    Transfer = 4,
    /// AT+CHLD=5: User-to-User Signaling.
    UserToUserSignaling = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallHoldParam {
    pub op: CallHoldAction,
    pub call_id: Option<u8>,
}

impl<'a> Parsable<'a> for CallHoldParam {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        let (op_val, call_id) = if val >= 100 {
            (val / 100, Some(val % 100))
        } else if val >= 10 {
            (val / 10, Some(val % 10))
        } else {
            (val, None)
        };
        let op = match op_val {
            0 => CallHoldAction::ReleaseHeld,
            1 => CallHoldAction::ReleaseAndAccept,
            2 => CallHoldAction::HoldAndAccept,
            3 => CallHoldAction::Conference,
            4 => CallHoldAction::Transfer,
            5 => CallHoldAction::UserToUserSignaling,
            _ => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::MapRes,
                )));
            }
        };
        Ok((input, Self { op, call_id }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FacilityLockMode {
    Unlock = 0,
    Lock = 1,
    QueryStatus = 2,
}

impl<'a> Parsable<'a> for FacilityLockMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Unlock)),
            1 => Ok((input, Self::Lock)),
            2 => Ok((input, Self::QueryStatus)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SmsBroadcastMode {
    #[default]
    Accept = 0,
    Discard = 1,
}

impl<'a> Parsable<'a> for SmsBroadcastMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Accept)),
            1 => Ok((input, Self::Discard)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ProductSerialNumberType {
    #[default]
    ImeiWithInfo = 0,
    Imei = 1,
    ImeiWithSvn = 2,
    Svn = 3,
}

impl<'a> Parsable<'a> for ProductSerialNumberType {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::ImeiWithInfo)),
            1 => Ok((input, Self::Imei)),
            2 => Ok((input, Self::ImeiWithSvn)),
            3 => Ok((input, Self::Svn)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IcfFormat {
    Data8Stop2 = 1,
    Data8Parity1Stop1 = 2,
    Data8Stop1 = 3,
    Data7Stop2 = 4,
    Data7Parity1Stop1 = 5,
    Data7Stop1 = 6,
}

impl<'a> Parsable<'a> for IcfFormat {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            1 => Ok((input, Self::Data8Stop2)),
            2 => Ok((input, Self::Data8Parity1Stop1)),
            3 => Ok((input, Self::Data8Stop1)),
            4 => Ok((input, Self::Data7Stop2)),
            5 => Ok((input, Self::Data7Parity1Stop1)),
            6 => Ok((input, Self::Data7Stop1)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IcfParity {
    Odd = 0,
    Even = 1,
    Mark = 2,
    Space = 3,
}

impl<'a> Parsable<'a> for IcfParity {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Odd)),
            1 => Ok((input, Self::Even)),
            2 => Ok((input, Self::Mark)),
            3 => Ok((input, Self::Space)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FlowControlMode {
    None = 0,
    XonXoff = 1,
    Hardware = 2,
}

impl<'a> Parsable<'a> for FlowControlMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::None)),
            1 => Ok((input, Self::XonXoff)),
            2 => Ok((input, Self::Hardware)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CallMode {
    SingleMode = 0,
    AlternateVoiceData = 1,
}

impl<'a> Parsable<'a> for CallMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::SingleMode)),
            1 => Ok((input, Self::AlternateVoiceData)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CdmaSubscriptionSource {
    #[default]
    RuimSim = 0,
    Nv = 1,
}

impl<'a> Parsable<'a> for CdmaSubscriptionSource {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::RuimSim)),
            1 => Ok((input, Self::Nv)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ClipActivation {
    #[default]
    Disable = 0,
    Enable = 1,
}

impl<'a> Parsable<'a> for ClipActivation {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Disable)),
            1 => Ok((input, Self::Enable)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipProvisionStatus {
    NotProvisioned = 0,
    Provisioned = 1,
    Unknown = 2,
}

impl<'a> Parsable<'a> for ClipProvisionStatus {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::NotProvisioned)),
            1 => Ok((input, Self::Provisioned)),
            2 => Ok((input, Self::Unknown)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CallWaitingPresentation {
    Disable = 0,
    Enable = 1,
}

impl<'a> Parsable<'a> for CallWaitingPresentation {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Disable)),
            1 => Ok((input, Self::Enable)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormattedNumber<'a> {
    pub number: &'a str,
    pub toa: TypeOfAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum NumberPresentation {
    #[default]
    Allowed = 0,
    Restricted = 1,
    NotAvailable = 2,
}

impl NumberPresentation {
    pub fn format_number<'a>(&self, number: Option<&'a PhoneNumber>) -> FormattedNumber<'a> {
        match self {
            Self::Restricted | Self::NotAvailable => {
                FormattedNumber { number: "", toa: TypeOfAddress::National }
            }
            Self::Allowed => {
                if let Some(num) = number {
                    FormattedNumber { number: num.as_str(), toa: num.toa() }
                } else {
                    FormattedNumber { number: "", toa: TypeOfAddress::National }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CallWaitingMode {
    Disable = 0,
    Enable = 1,
    Query = 2,
}

impl<'a> Parsable<'a> for CallWaitingMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Disable)),
            1 => Ok((input, Self::Enable)),
            2 => Ok((input, Self::Query)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CallWaitingStatus {
    NotActive = 0,
    Active = 1,
}

impl<'a> Parsable<'a> for CallWaitingStatus {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::NotActive)),
            1 => Ok((input, Self::Active)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

impl<'a> Parsable<'a> for bool {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, false)),
            1 => Ok((input, true)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ClirMode {
    #[default]
    SubscriptionDefault = 0,
    Invocation = 1,
    Suppression = 2,
}

impl<'a> Parsable<'a> for ClirMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::SubscriptionDefault)),
            1 => Ok((input, Self::Invocation)),
            2 => Ok((input, Self::Suppression)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CallForwardingReason {
    Unconditional = 0,
    Busy = 1,
    NoReply = 2,
    NotReachable = 3,
    All = 4,
    AllConditional = 5,
}

impl<'a> Parsable<'a> for CallForwardingReason {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Unconditional)),
            1 => Ok((input, Self::Busy)),
            2 => Ok((input, Self::NoReply)),
            3 => Ok((input, Self::NotReachable)),
            4 => Ok((input, Self::All)),
            5 => Ok((input, Self::AllConditional)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CallForwardingMode {
    Disable = 0,
    Enable = 1,
    Query = 2,
    Registration = 3,
    Erasure = 4,
}

impl<'a> Parsable<'a> for CallForwardingMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Disable)),
            1 => Ok((input, Self::Enable)),
            2 => Ok((input, Self::Query)),
            3 => Ok((input, Self::Registration)),
            4 => Ok((input, Self::Erasure)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UssdMode {
    DisableUrc = 0,
    EnableUrc = 1,
    Cancel = 2,
}

impl<'a> Parsable<'a> for UssdMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::DisableUrc)),
            1 => Ok((input, Self::EnableUrc)),
            2 => Ok((input, Self::Cancel)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ClirStatus {
    NotActive = 0,
    Active = 1,
    Unknown = 2,
    TemporaryRestricted = 3,
    TemporaryAllowed = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UssdStatus {
    NoActionRequired = 0,
    ActionRequired = 1,
    TerminatedByNetwork = 2,
    OtherClientResponded = 3,
    NotSupported = 4,
    NetworkTimeout = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SpeakerMuteMode {
    Off = 0,
    #[default]
    OnAndOffOnCarrier = 1,
    AlwaysOn = 2,
    OffDialRingOnConnectOffCarrier = 3,
}

impl<'a> Parsable<'a> for SpeakerMuteMode {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Off)),
            1 => Ok((input, Self::OnAndOffOnCarrier)),
            2 => Ok((input, Self::AlwaysOn)),
            3 => Ok((input, Self::OffDialRingOnConnectOffCarrier)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CdmaRoamingPreference {
    #[default]
    HomeOnly = 0,
    RoamingOnly = 1,
    Automatic = 2,
}

impl<'a> Parsable<'a> for CdmaRoamingPreference {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::HomeOnly)),
            1 => Ok((input, Self::RoamingOnly)),
            2 => Ok((input, Self::Automatic)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdpType {
    Ip,
    Ipv6,
    Ipv4v6,
    Ppp,
    NonIp,
    Cell,
}

impl<'a> Parsable<'a> for PdpType {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, quoted) = QuotedString::parse(input)?;
        match quoted.as_ref() {
            b"IP" => Ok((input, Self::Ip)),
            b"IPV6" => Ok((input, Self::Ipv6)),
            b"IPV4V6" => Ok((input, Self::Ipv4v6)),
            b"PPP" => Ok((input, Self::Ppp)),
            b"Non-IP" => Ok((input, Self::NonIp)),
            b"Cell" => Ok((input, Self::Cell)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

impl std::fmt::Display for PdpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdpType::Ip => write!(f, "IP"),
            PdpType::Ipv6 => write!(f, "IPV6"),
            PdpType::Ipv4v6 => write!(f, "IPV4V6"),
            PdpType::Ppp => write!(f, "PPP"),
            PdpType::NonIp => write!(f, "Non-IP"),
            PdpType::Cell => write!(f, "Cell"),
        }
    }
}

impl std::fmt::Display for CopsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CopsFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for RegistrationUnsolicitedMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for RadioPowerLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CmeeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CallHoldAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for FacilityLockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for SmsBroadcastMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for ProductSerialNumberType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for IcfFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for IcfParity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for FlowControlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CallMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CdmaSubscriptionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for ClipActivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CallWaitingPresentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CallWaitingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CallWaitingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for ClirMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CallForwardingReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CallForwardingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for UssdMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for SpeakerMuteMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for ClirStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for ClipProvisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for UssdStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

impl std::fmt::Display for CdmaRoamingPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AccessTechnology {
    Gsm = 0,
    Wcdma = 2,
    #[default]
    Lte = 7,
    Nr = 11,
}

impl<'a> Parsable<'a> for AccessTechnology {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Gsm)),
            2 => Ok((input, Self::Wcdma)),
            7 => Ok((input, Self::Lte)),
            11 => Ok((input, Self::Nr)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

impl std::fmt::Display for AccessTechnology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CtecTechnology {
    Gsm = 1,
    Wcdma = 2,
    #[default]
    Lte = 32,
    Nr = 64,
}

impl<'a> Parsable<'a> for CtecTechnology {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            1 => Ok((input, Self::Gsm)),
            2 => Ok((input, Self::Wcdma)),
            32 => Ok((input, Self::Lte)),
            64 => Ok((input, Self::Nr)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

impl std::fmt::Display for CtecTechnology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facility {
    SimPin,
    FixedDial,
    Other,
}

impl<'a> Parsable<'a> for Facility {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, quoted) = QuotedString::parse(input)?;
        match quoted.as_ref() {
            b"SC" => Ok((input, Self::SimPin)),
            b"FD" => Ok((input, Self::FixedDial)),
            _ => Ok((input, Self::Other)),
        }
    }
}

impl std::fmt::Display for Facility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Facility::SimPin => write!(f, "SC"),
            Facility::FixedDial => write!(f, "FD"),
            Facility::Other => write!(f, "OTHER"),
        }
    }
}

impl<'a> Parsable<'a> for crate::apdu::Instruction {
    fn parse(input: &'a [u8]) -> nom::IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        Ok((input, crate::apdu::Instruction::from(val)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::UiccFileId;

    #[test]
    fn test_phone_number_parse() {
        let (rem, phone) = PhoneNumber::parse(b"+16505550100").unwrap();
        assert_eq!(rem, b"");
        assert_eq!(phone.as_str(), "+16505550100");

        let (rem, phone) = PhoneNumber::parse(b"12345*678#").unwrap();
        assert_eq!(rem, b"");
        assert_eq!(phone.as_str(), "12345*678#");

        // Prefix parsing behavior: stops at first invalid char
        let (rem, phone) = PhoneNumber::parse(b"123a45").unwrap();
        assert_eq!(rem, b"a45");
        assert_eq!(phone.as_str(), "123");

        // Plus at non-start is invalid and ends digits matching
        let (rem, phone) = PhoneNumber::parse(b"12+34").unwrap();
        assert_eq!(rem, b"+34");
        assert_eq!(phone.as_str(), "12");
    }

    #[test]
    fn test_dial_string_clean_number() {
        // Valid
        let dial = DialString::parse(b"12345").unwrap();
        assert_eq!(dial.clean_number().unwrap().as_str(), "12345");

        let dial = DialString::parse(b"+16505550100").unwrap();
        assert_eq!(dial.clean_number().unwrap().as_str(), "+16505550100");

        // Strips CLIR and modifiers
        let dial = DialString::parse(b"12345i,1234").unwrap();
        assert_eq!(dial.clean_number().unwrap().as_str(), "12345");

        let dial = DialString::parse(b"+12345I,678").unwrap();
        assert_eq!(dial.clean_number().unwrap().as_str(), "+12345");

        // Strictly rejects invalid dial characters (like 'a')
        assert!(DialString::parse(b"123a45").is_none());
        assert!(DialString::parse(b"123b45").is_none());
        assert!(DialString::parse(b"123c45").is_none());
    }

    #[test]
    fn test_dial_string_clir() {
        let dial = DialString::parse(b"12345i").unwrap();
        assert_eq!(dial.clir(ClirMode::SubscriptionDefault), ClirMode::Suppression);

        let dial = DialString::parse(b"12345I").unwrap();
        assert_eq!(dial.clir(ClirMode::SubscriptionDefault), ClirMode::Invocation);

        let dial = DialString::parse(b"12345").unwrap();
        assert_eq!(dial.clir(ClirMode::SubscriptionDefault), ClirMode::SubscriptionDefault);
        assert_eq!(dial.clir(ClirMode::Invocation), ClirMode::Invocation);

        // Modifiers with pause/wait
        let dial = DialString::parse(b"12345i,1234").unwrap();
        assert_eq!(dial.clir(ClirMode::SubscriptionDefault), ClirMode::Suppression);

        // Number suffix with @ parameters where @ part has no CLIR suffix
        let dial = DialString::parse(b"12345i@1,2").unwrap();
        assert_eq!(dial.clir(ClirMode::SubscriptionDefault), ClirMode::Suppression);

        let dial = DialString::parse(b"12345I@1,2").unwrap();
        assert_eq!(dial.clir(ClirMode::SubscriptionDefault), ClirMode::Invocation);

        // Emergency dial string with CLIR suffix after @
        let dial = DialString::parse(b"911@1,#i").unwrap();
        assert_eq!(dial.clir(ClirMode::SubscriptionDefault), ClirMode::Suppression);
    }

    #[test]
    fn test_dial_args_parse() {
        let (rem, args) = DialArgs::parse(b"12345i;\r\n").unwrap();
        assert_eq!(rem, b"\r\n");
        assert_eq!(args.number.as_str(), "12345");
        assert_eq!(args.clir, ClirMode::Suppression);
        assert!(!args.is_emergency);

        let (rem, args) = DialArgs::parse(b"12345I\r\n").unwrap();
        assert_eq!(rem, b"\r\n");
        assert_eq!(args.number.as_str(), "12345");
        assert_eq!(args.clir, ClirMode::Invocation);
        assert!(!args.is_emergency);

        let (rem, args) = DialArgs::parse(b"911;\r\n").unwrap();
        assert_eq!(rem, b"\r\n");
        assert_eq!(args.number.as_str(), "911");
        assert!(args.is_emergency);
    }

    #[test]
    fn test_phone_number_is_gprs_dial() {
        assert!(PhoneNumber::new("*99#").is_gprs_dial());
        assert!(PhoneNumber::new("*99*1#").is_gprs_dial());
        assert!(PhoneNumber::new("*99***1#").is_gprs_dial());
        assert!(!PhoneNumber::new("12345").is_gprs_dial());
        assert!(!PhoneNumber::new("*99").is_gprs_dial());
        assert!(!PhoneNumber::new("*99#1").is_gprs_dial());
    }

    #[test]
    fn test_phone_number_toa() {
        assert_eq!(PhoneNumber::new("+16505550100").toa(), TypeOfAddress::International);
        assert_eq!(PhoneNumber::new("16505550100").toa(), TypeOfAddress::National);
        assert_eq!(PhoneNumber::new("12345").toa(), TypeOfAddress::National);
    }

    #[test]
    fn test_number_presentation_format_number() {
        let phone = PhoneNumber::new("12345");

        // Allowed
        let formatted = NumberPresentation::Allowed.format_number(Some(&phone));
        assert_eq!(formatted.number, "12345");
        assert_eq!(formatted.toa, TypeOfAddress::National);

        let int_phone = PhoneNumber::new("+12345");
        let formatted = NumberPresentation::Allowed.format_number(Some(&int_phone));
        assert_eq!(formatted.number, "+12345");
        assert_eq!(formatted.toa, TypeOfAddress::International);

        let formatted = NumberPresentation::Allowed.format_number(None);
        assert_eq!(formatted.number, "");
        assert_eq!(formatted.toa, TypeOfAddress::National);

        // Restricted
        let formatted = NumberPresentation::Restricted.format_number(Some(&phone));
        assert_eq!(formatted.number, "");
        assert_eq!(formatted.toa, TypeOfAddress::National);

        // Not Available
        let formatted = NumberPresentation::NotAvailable.format_number(Some(&phone));
        assert_eq!(formatted.number, "");
        assert_eq!(formatted.toa, TypeOfAddress::National);
    }

    #[test]
    fn test_adn_record_encode_from_number() {
        let record_len = UiccFileId::Msisdn.default_record_len().expect("file id is record based");
        let phone = PhoneNumber::new("+15555215554");
        let record = AdnRecord::encode_from_number(&phone, record_len);
        assert_eq!(record.len(), record_len);
        // Alpha identifier is padded with 0xFF
        assert_eq!(&record[..14], &[0xFF; 14]);
        // Length of BCD number is 7 (1 byte TON + 6 bytes dialed digits)
        assert_eq!(record[14], 7);
        // International TON/NPI
        assert_eq!(record[15], 0x91);
        // Dialing digits 15555215554 -> 51 55 25 51 55 F4
        assert_eq!(&record[16..22], &[0x51, 0x55, 0x25, 0x51, 0x55, 0xF4]);
        // Trailing dialing bytes padded with 0xFF
        assert_eq!(&record[22..26], &[0xFF; 4]);
        // Capability/Extension bytes
        assert_eq!(&record[26..28], &[0xFF, 0xFF]);

        // Empty digits returns standard unassigned 0xFF record
        let empty = PhoneNumber::new("");
        assert_eq!(AdnRecord::encode_from_number(&empty, 28), vec![0xFF; 28]);
    }

    #[test]
    fn test_adn_record_cts_mbdn_roundtrip() {
        let cts_hex =
            "74616741FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF06812143658709FFFFFFFFFFFFFF";
        let raw_bytes = hex::decode(cts_hex).unwrap();
        assert_eq!(raw_bytes.len(), 38);

        let decoded = AdnRecord::decode(&raw_bytes).expect("Should decode CTS MBDN record");
        assert_eq!(decoded.alpha_tag.as_deref(), Some("tagA"));
        assert_eq!(decoded.number.as_ref().map(|p| p.as_str()), Some("1234567890"));

        let reencoded = decoded.encode(38);
        assert_eq!(hex::encode_upper(&reencoded), cts_hex);
    }

    #[test]
    fn test_adn_record_msisdn_with_alpha_tag() {
        let record = AdnRecord::new(Some("MySIM"), Some(PhoneNumber::new("+15551234567")));
        let encoded = record.encode(28);
        assert_eq!(encoded.len(), 28);

        let decoded = AdnRecord::decode(&encoded).expect("Should decode MSISDN record");
        assert_eq!(decoded.alpha_tag.as_deref(), Some("MySIM"));
        assert_eq!(decoded.number.as_ref().map(|p| p.as_str()), Some("+15551234567"));
    }

    #[test]
    fn test_adn_record_service_code() {
        let record = AdnRecord::new(Some("Voicemail"), Some(PhoneNumber::new("*86")));
        let encoded = record.encode(38);
        let decoded = AdnRecord::decode(&encoded).expect("Should decode service code record");
        assert_eq!(decoded.alpha_tag.as_deref(), Some("Voicemail"));
        assert_eq!(decoded.number.as_ref().map(|p| p.as_str()), Some("*86"));
    }

    #[test]
    fn test_adn_record_uninitialized() {
        assert!(AdnRecord::decode(&[0xFF; 28]).is_none());
        assert!(AdnRecord::decode(&[0xFF; 38]).is_none());
        assert!(AdnRecord::decode(&[0xFF; 10]).is_none()); // malformed < 14
    }

    #[test]
    fn test_adn_record_long_number_truncation() {
        let long_number = PhoneNumber::new("123456789012345678901234");
        let record = AdnRecord::new(Some("Long"), Some(long_number));
        let encoded = record.encode(28);

        let decoded = AdnRecord::decode(&encoded).expect("Should decode truncated record");
        assert_eq!(decoded.alpha_tag.as_deref(), Some("Long"));
        assert_eq!(decoded.number.as_ref().map(|p| p.as_str()), Some("12345678901234567890"));
    }

    #[test]
    fn test_adn_record_length_boundaries() {
        // Less than 14 bytes returns None
        assert!(AdnRecord::decode(&[]).is_none());
        assert!(AdnRecord::decode(&[0xFF; 13]).is_none());
        assert!(AdnRecord::decode(&[0x06, 0x81, 0x21, 0x43, 0x65, 0x87, 0x09]).is_none());

        // encode() with len < 14 returns 0xFF buffer
        let rec = AdnRecord::new(Some("A"), Some(PhoneNumber::new("123")));
        assert_eq!(rec.encode(0), Vec::<u8>::new());
        assert_eq!(rec.encode(13), vec![0xFF; 13]);

        // Exactly 14 bytes (alpha_len == 0)
        let rec_14 = AdnRecord::new(None::<&str>, Some(PhoneNumber::new("+15551234567")));
        let encoded_14 = rec_14.encode(14);
        assert_eq!(encoded_14.len(), 14);
        let decoded_14 = AdnRecord::decode(&encoded_14).unwrap();
        assert_eq!(decoded_14.alpha_tag, None);
        assert_eq!(decoded_14.number.as_ref().map(|p| p.as_str()), Some("+15551234567"));

        // Exactly 14 bytes with alpha_tag truncates alpha to 0 bytes cleanly
        let rec_with_tag = AdnRecord::new(Some("Tag"), Some(PhoneNumber::new("123456")));
        let encoded_14_tag = rec_with_tag.encode(14);
        let decoded_14_tag = AdnRecord::decode(&encoded_14_tag).unwrap();
        assert_eq!(decoded_14_tag.alpha_tag, None);
        assert_eq!(decoded_14_tag.number.as_ref().map(|p| p.as_str()), Some("123456"));
    }

    #[test]
    fn test_adn_record_odd_vs_even_digit_padding() {
        // Odd digits: 7 digits ("1234567") -> 4 BCD bytes: 21 43 65 F7
        let rec_odd = AdnRecord::new(None::<&str>, Some(PhoneNumber::new("1234567")));
        let encoded_odd = rec_odd.encode(28);
        assert_eq!(encoded_odd[14], 5); // 1 TON + 4 BCD bytes
        assert_eq!(encoded_odd[15], 0x81);
        assert_eq!(&encoded_odd[16..20], &[0x21, 0x43, 0x65, 0xF7]);
        assert_eq!(&encoded_odd[20..26], &[0xFF; 6]); // Unused 6 bytes in 10-byte buffer
        let decoded_odd = AdnRecord::decode(&encoded_odd).unwrap();
        assert_eq!(decoded_odd.number.as_ref().map(|p| p.as_str()), Some("1234567"));

        // Even digits: 8 digits ("12345678") -> 4 BCD bytes: 21 43 65 87
        let rec_even = AdnRecord::new(None::<&str>, Some(PhoneNumber::new("12345678")));
        let encoded_even = rec_even.encode(28);
        assert_eq!(encoded_even[14], 5);
        assert_eq!(&encoded_even[16..20], &[0x21, 0x43, 0x65, 0x87]);
        assert_eq!(&encoded_even[20..26], &[0xFF; 6]);
        let decoded_even = AdnRecord::decode(&encoded_even).unwrap();
        assert_eq!(decoded_even.number.as_ref().map(|p| p.as_str()), Some("12345678"));

        // Single digit ("5") -> 1 BCD byte: F5, len 2
        let rec_single = AdnRecord::new(None::<&str>, Some(PhoneNumber::new("5")));
        let encoded_single = rec_single.encode(28);
        assert_eq!(encoded_single[14], 2);
        assert_eq!(encoded_single[16], 0xF5);
        let decoded_single = AdnRecord::decode(&encoded_single).unwrap();
        assert_eq!(decoded_single.number.as_ref().map(|p| p.as_str()), Some("5"));
    }

    #[test]
    fn test_adn_record_length_byte_edge_cases() {
        // len_byte == 0 returns number: None
        let mut raw = vec![0xFF; 28];
        raw[..4].copy_from_slice(b"Test");
        raw[14] = 0x00;
        let dec = AdnRecord::decode(&raw).unwrap();
        assert_eq!(dec.alpha_tag.as_deref(), Some("Test"));
        assert_eq!(dec.number, None);

        // len_byte == 1 (TON only, 0 BCD digits) returns number: None
        raw[14] = 0x01;
        raw[15] = 0x81;
        let dec = AdnRecord::decode(&raw).unwrap();
        assert_eq!(dec.number, None);

        // len_byte > 11 (malformed length byte) returns number: None without panic
        raw[14] = 12;
        assert_eq!(AdnRecord::decode(&raw).unwrap().number, None);
        raw[14] = 254;
        assert_eq!(AdnRecord::decode(&raw).unwrap().number, None);
    }

    #[test]
    fn test_adn_record_illegal_bcd_nibble_rejection() {
        // Record with illegal nibbles 0xC, 0xD, 0xE in BCD digits
        let mut raw = vec![0xFF; 28];
        raw[..4].copy_from_slice(b"Test");
        raw[14] = 3; // 1 TON + 2 BCD bytes
        raw[15] = 0x81;
        raw[16] = 0xC1; // '1' and 'a'
        raw[17] = 0x32; // '2' and '3'
        // Decoded string would be "1a23", which PhoneNumber::parse rejects
        let dec = AdnRecord::decode(&raw).unwrap();
        assert_eq!(dec.number, None);
    }

    #[test]
    fn test_adn_record_alpha_tag_edge_cases() {
        // Full alpha field with no trailing 0xFF (exactly 14 characters)
        let rec = AdnRecord::new(Some("12345678901234"), Some(PhoneNumber::new("999")));
        let enc = rec.encode(28);
        assert_eq!(&enc[..14], b"12345678901234");
        let dec = AdnRecord::decode(&enc).unwrap();
        assert_eq!(dec.alpha_tag.as_deref(), Some("12345678901234"));

        // Alpha field with spaces preserved
        let rec_spaces = AdnRecord::new(Some("John Doe "), Some(PhoneNumber::new("999")));
        let enc_spaces = rec_spaces.encode(28);
        let dec_spaces = AdnRecord::decode(&enc_spaces).unwrap();
        assert_eq!(dec_spaces.alpha_tag.as_deref(), Some("John Doe "));
    }

    #[test]
    fn test_adn_record_large_buffer() {
        // 50-byte record (alpha_len = 50 - 14 = 36 bytes)
        let rec = AdnRecord::new(
            Some("Alpha tag in 50-byte record"),
            Some(PhoneNumber::new("+15550001111")),
        );
        let enc = rec.encode(50);
        assert_eq!(enc.len(), 50);
        let dec = AdnRecord::decode(&enc).unwrap();
        assert_eq!(dec.alpha_tag.as_deref(), Some("Alpha tag in 50-byte record"));
        assert_eq!(dec.number.as_ref().map(|p| p.as_str()), Some("+15550001111"));
    }
}
