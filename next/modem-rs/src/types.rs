use std::{fmt, str};

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
pub const DEFAULT_PUK: &str = "12345678";
pub const DEFAULT_IP_ADDRESS: &str = "192.168.1.1";

// A unique identifier for a modem instance.
pub type ModemId = u32;

// Custom error type for the library.
#[derive(Debug)]
pub enum ModemError {
    DuplicateModemId(ModemId),
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
    ReceiveSms(Vec<u8>),
    ReceiveTextSms { to: String, text: String },
    None,
}

/// Callbacks for a single modem instance.
pub trait Callbacks: Send + Sync {
    /// Sends an AT response to the modem.
    fn send_at_response(&self, modem_id: ModemId, response: &[u8]);
}

/// An extension trait for `Callbacks` that provides helper methods.
pub trait CallbacksExt: Callbacks {
    /// Sends a response followed by "OK".
    fn send_response_ok(&self, modem_id: ModemId, response: &[u8]) {
        self.send_at_response(modem_id, response);
        self.send_ok(modem_id);
    }

    /// Sends an "OK" response.
    fn send_ok(&self, modem_id: ModemId) {
        self.send_at_response(modem_id, AT_OK);
    }

    /// Sends an "ERROR" response.
    fn send_error(&self, modem_id: ModemId) {
        self.send_at_response(modem_id, AT_ERROR);
    }

    /// Formats a response and sends it.
    fn send_formatted(&self, modem_id: ModemId, args: fmt::Arguments) {
        let response = fmt::format(args).into_bytes();
        self.send_at_response(modem_id, &response);
    }

    /// Formats a response and sends it followed by "OK".
    fn send_formatted_ok(&self, modem_id: ModemId, args: fmt::Arguments) {
        let response = fmt::format(args).into_bytes();
        self.send_response_ok(modem_id, &response);
    }
}

impl<T: Callbacks + ?Sized> CallbacksExt for T {}

/// Callbacks for the cellular network simulator.
pub trait NetworkCallbacks: Send + Sync {
    /// Called when a new remote connection is established.
    fn on_new_remote_connection(&self, modem_id: ModemId, destination: String);

    /// Called when a modem hangs up a call.
    fn on_modem_hanged_up(&self, modem_id: ModemId);
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
