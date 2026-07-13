// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{str, time::Duration};

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

pub const CME_NO_RESOURCES: u32 = 142;

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

    /// Creates a result with a specific "+CME ERROR" response and no follow-up
    /// action.
    pub fn cme_error(code: u32) -> Self {
        Self { responses: vec![format!("+CME ERROR: {}\r\n", code)], action: None }
    }
}

/// Represents the outcome of a command execution from the new parser.
pub enum ExecutionResult {
    /// The command was successfully handled, yielding a response and/or action.
    /// This covers both "OK" and "ERROR" responses.
    Handled(HandledCommand),

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
            lte_rssi: unknown,
            lte_rsrp: max,
            lte_rsrq: max,
            lte_rssnr: max,
            lte_cqi: max,
            lte_ta: max,
            tdscdma_rscp: max,
            wcdma_rssi: unknown,
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
