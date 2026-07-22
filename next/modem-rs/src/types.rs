// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{str, time::Duration};

use netsim_model::Quirks;
use nom::IResult;

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

pub const DEFAULT_GATEWAY: &str = "10.0.2.2";
pub const DEFAULT_DNS: &str = "10.0.2.3";

// A unique identifier for a modem instance.
pub type ModemId = u32;

// Custom error type for the library.
use std::fmt;

#[derive(Debug, Clone)]
pub enum ModemError {
    DuplicateModemId(ModemId),
    NotFound,
}

impl std::error::Error for ModemError {}

impl fmt::Display for ModemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModemError::DuplicateModemId(id) => write!(f, "Duplicate modem ID: {id}"),
            ModemError::NotFound => write!(f, "Modem network not found"),
        }
    }
}

// Actions that a command can request to be executed by the
// CellularNetworkSimulator.
#[derive(Debug, PartialEq)]
pub enum CommandAction {
    InitiateCall(String),
    InitiateRemoteCall(String),
    InitiateEmergencyCall,
    AnswerCall(ModemId),
    HangupCall(ModemId),
    InitiateCallAndHold(String),
    SwapCalls(ModemId, ModemId),
    ReceiveSms { to: Option<String>, pdu: Vec<u8> },
    ReceiveTextSms { to: String, text: String },
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

// --- New types for architectural refactoring ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CmeeMode {
    #[default]
    Disable = 0, // Returns "ERROR"
    Numeric = 1, // Returns "+CME ERROR: <code>"
    Verbose = 2, // Returns "+CME ERROR: <verbose string>"
}

impl CmeeMode {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Disable),
            1 => Some(Self::Numeric),
            2 => Some(Self::Verbose),
            _ => None,
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

/// Contains all the results of a successfully executed command.
#[derive(Default)]
pub struct HandledCommand {
    /// The immediate response to send back to the client (e.g., "OK\r\n").
    pub responses: Vec<String>,
    /// An optional follow-up action for the CellularNetworkSimulator to
    /// perform.
    pub action: Option<CommandAction>,
}

impl HandledCommand {
    /// Creates a result with a simple "OK" response and no follow-up action.
    pub fn ok() -> Self {
        Self { responses: vec!["OK\r\n".to_string()], action: None }
    }

    /// Creates a result with a simple "OK" response AND a follow-up action.
    pub fn ok_with_action(action: CommandAction) -> Self {
        Self { responses: vec!["OK\r\n".to_string()], action: Some(action) }
    }
    /// Creates a result with a simple "ERROR" response and no follow-up action.
    pub fn error() -> Self {
        Self { responses: vec!["ERROR\r\n".to_string()], action: None }
    }
}

/// Represents the outcome of a command execution from the new parser.
pub enum ExecutionResult {
    /// The command was successfully handled, yielding a response and/or action.
    Success(HandledCommand),

    /// The command failed with a standard generic AT error ("ERROR\r\n").
    Error,

    /// The command failed with a standard generic AT error ("ERROR\r\n"), after
    /// emitting one or more URC strings.
    ErrorWithUrc(Vec<String>),

    /// The command failed with a structured Mobile Equipment (ME) error.
    CmeError(CmeError),

    /// The command failed with a structured Mobile Equipment (ME) error, after
    /// emitting one or more URC strings.
    CmeErrorWithUrc(CmeError, Vec<String>),

    /// This command has not been refactored yet and should be handled by the
    /// legacy system.
    Unhandled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEvent {
    SinkError(u32),
    TimerRequest { chip_id: u32, duration: Duration },
}

#[derive(Debug, Clone, Default)]
pub struct ModemInfo {
    pub id: u32,
    pub connections: Vec<String>, // Placeholder for actual connection info
    pub ringing: bool,
    pub sms_count: usize,
    pub quirks: Quirks,
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
