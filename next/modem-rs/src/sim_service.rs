// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use tracing::warn;

use crate::{
    config::{DedicatedFile, ElementaryFile, FileSystem, SimFile, SimProfile},
    parser::{Command, QuotedString},
    types::{CmeError, DEFAULT_PIN, ExecutionResult, HandledCommand},
};

const DEFAULT_PUK: &str = "12345678";

const APDU_SELECT: u16 = 0xA4;
const APDU_READ_BINARY: u16 = 0xB0;
const APDU_READ_RECORD: u16 = 0xB2;
const APDU_GET_RESPONSE: u16 = 0xC0;

const DEFAULT_FALLBACK_IMSI: &str = "310260123456789";
const DEFAULT_FALLBACK_ICCID: &str = "89012608640220133897";

mod iccprofile_for_sim0 {
    // --- EF_DIR (Directory File, ID: 0x2F00) ---
    // Contains the list of applications available on the SIM (USIM, CSIM, etc.)

    /// File Control Parameters (FCP) template for EF_DIR.
    /// Indicates Linear Fixed structure, record length 0x30 (48 bytes), 4
    /// records.
    pub const EF_DIR_FCP: &str = "621A8205422100300483022F008A01058B032F0601800200C08801F0";

    /// EF_DIR Record 1: CSIM Application template (AID:
    /// A0000003431002FF86FF0389FFFFFFFF, Label: "CSIM")
    pub const EF_DIR_RECORD_CSIM: &str = "61184F10A0000003431002FF86FF0389FFFFFFFF50044353494DFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF";

    /// EF_DIR Record 2: USIM Application template (AID:
    /// A0000000871002FF86FF0389FFFFFFFF, Label: "USIM")
    pub const EF_DIR_RECORD_USIM: &str = "61184F10A0000000871002FF86FF0389FFFFFFFF50045553494DFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF";

    /// Empty record for EF_DIR.
    pub const EF_DIR_RECORD_EMPTY: &str = "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF";

    // --- EF_ICCID (Integrated Circuit Card Identifier, ID: 0x2FE2) ---

    /// FCP template for EF_ICCID. Transparent structure, size 10 bytes.
    pub const EF_ICCID_FCP: &str = "62178202412183022FE28A01058B032F06038002000A880110";

    // --- EF_AD (Administrative Data, ID: 0x6FAD) ---

    /// FCP template for EF_AD. Transparent structure, size 4 bytes.
    pub const EF_AD_FCP: &str = "62178202412183026FAD8A01058B036F060180020004880118";

    /// Default data for EF_AD. Value `00000003` indicates Normal Operation and
    /// 3-digit MNC.
    pub const EF_AD_DATA_FALLBACK: &str = "00000003";

    // --- EF_MSISDN (Mobile Station International Subscriber Directory Number, ID:
    // 0x6F40) ---

    /// FCP template for EF_MSISDN. Linear Fixed structure, record length 28
    /// bytes, 2 records.
    pub const EF_MSISDN_FCP: &str = "621982054221001C0283026F408A01058B036F0605800200388800";

    /// Default mocked MSISDN record.
    pub const EF_MSISDN_RECORD_FALLBACK: &str =
        "000000000000000000000000000007915155214365F7FFFFFFFFFFFF";

    // --- PKCS15 Application (DF_TELECOM / DF_PHONEBOOK helper) ---
    // Used for logical channel transmission mocks (AT+CGLA)

    /// Response to SELECT PKCS15 AID (indicates 36 bytes available for GET
    /// RESPONSE).
    pub const PKCS15_SELECT_RESP: &str = "4,6124";

    /// FCP template response for PKCS15 (Get Response).
    pub const PKCS15_FCP_RESP: &str =
        "76,62228202412183025031A503C001408A01058B066F0601010001800200108102002288009000";

    /// Read Binary response for PKCS15.
    pub const PKCS15_READ_RESP: &str = "36,A706300404024401A5063004040244029000";

    /// Error response for PKCS15 (Status 6B00: Incorrect parameters P1-P2).
    pub const PKCS15_ERROR_RESP: &str = "4,6b00";
}

// Represents the state of the SIM card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimState {
    Absent,
    Ready,
    PinRequired,
    PukRequired,
}

// Holds all state related to the SIM card.
pub struct SimService {
    state: SimState,
    pin_enabled: bool,
    imsi: String,
    iccid: String,
    msisdn: String,
    pin1: String,
    pin1_retries: u8,
    puk1_retries: u8,
    fs: FileSystem,
    sms_messages: HashMap<u8, Vec<u8>>,
    logical_channels: [bool; 4],
    cdma_subscription_source: u8,
    cdma_roaming_preference: u8,
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

        Self {
            state,
            pin_enabled,
            imsi,
            iccid,
            msisdn,
            pin1: DEFAULT_PIN.to_string(),
            pin1_retries: 3,
            puk1_retries: 10,
            fs: profile.sim_io.file_system.clone(),
            sms_messages: HashMap::new(),
            // Channel 0 is the basic channel and is always open by default.
            logical_channels: [true, false, false, false],
            cdma_subscription_source: 0,
            cdma_roaming_preference: 0,
        }
    }

    pub fn set_msisdn(&mut self, msisdn: &str) {
        self.msisdn = msisdn.to_string();
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

    pub fn read_sms(&self, index: u8) -> ExecutionResult {
        if !self.is_present() {
            return ExecutionResult::CmeError(CmeError::SimNotInserted);
        }
        if let Some(pdu) = self.sms_messages.get(&index) {
            let response = format!("+CMGR: 0,,{}\r\n{}\r\n", pdu.len(), hex::encode_upper(pdu));
            let mut handled = HandledCommand::ok();
            handled.responses.insert(0, response);
            ExecutionResult::Success(handled)
        } else {
            ExecutionResult::Error
        }
    }

    pub fn delete_sms(&mut self, index: u8) -> bool {
        if !self.is_present() {
            return false;
        }
        self.sms_messages.remove(&index).is_some()
    }

    // --- Pure command handlers ---

    fn handle_get_sim_status(&self) -> ExecutionResult {
        let response_str = match self.state {
            SimState::Absent => unreachable!("Absent state handled in execute"),
            SimState::Ready => "+CPIN: READY\r\n",
            SimState::PinRequired => "+CPIN: SIM PIN\r\n",
            SimState::PukRequired => "+CPIN: SIM PUK\r\n",
        };
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response_str.to_string());
        ExecutionResult::Success(handled)
    }

    fn handle_enter_pin(
        &mut self,
        pin_or_puk: QuotedString,
        new_pin: Option<QuotedString>,
    ) -> ExecutionResult {
        match self.state {
            SimState::Absent => unreachable!("Absent state handled in execute"),
            SimState::Ready => {
                if pin_or_puk.as_ref() == self.pin1.as_bytes() {
                    ExecutionResult::Success(HandledCommand::ok())
                } else {
                    self.pin1_retries -= 1;
                    if self.pin1_retries == 0 {
                        self.state = SimState::PukRequired;
                    }
                    ExecutionResult::CmeError(CmeError::IncorrectPassword)
                }
            }
            SimState::PinRequired => {
                if pin_or_puk.as_ref() == self.pin1.as_bytes() {
                    self.state = SimState::Ready;
                    ExecutionResult::Success(HandledCommand::ok())
                } else {
                    self.pin1_retries -= 1;
                    if self.pin1_retries == 0 {
                        self.state = SimState::PukRequired;
                    }
                    ExecutionResult::CmeError(CmeError::IncorrectPassword)
                }
            }
            SimState::PukRequired => {
                if let Some(new_pin) = new_pin {
                    if pin_or_puk.as_ref() == DEFAULT_PUK.as_bytes() {
                        let _ = new_pin;
                        self.state = SimState::Ready;
                        self.pin1_retries = 3;
                        self.puk1_retries = 10;
                        ExecutionResult::Success(HandledCommand::ok())
                    } else {
                        self.puk1_retries -= 1;
                        ExecutionResult::CmeError(CmeError::IncorrectPassword)
                    }
                } else {
                    ExecutionResult::CmeError(CmeError::IncorrectPassword)
                }
            }
        }
    }

    fn handle_get_imsi(&self) -> ExecutionResult {
        let response = format!("{}\r\n", self.imsi);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Success(handled)
    }

    fn handle_get_iccid(&self) -> ExecutionResult {
        let response = format!("{}\r\n", self.iccid);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Success(handled)
    }

    fn handle_sim_io(
        &self,
        command: u16,
        file_id: u16,
        p1: u8,
        _p2: u8,
        _p3: u8,
        _data: Option<QuotedString>,
    ) -> ExecutionResult {
        // TODO: Extract goldfish-specific quirks into flags.
        // 1. Try to read from the loaded FileSystem first (for READ BINARY and SELECT)
        if command == APDU_READ_BINARY {
            if let Some(ef) = find_ef(&self.fs.master_file, &format!("{file_id:04X}")) {
                return ExecutionResult::Success(HandledCommand {
                    responses: vec![format!("+CRSM: 144,0,{}\r\n", ef.data), "OK\r\n".to_string()],
                    action: None,
                });
            }
        } else if command == APDU_SELECT
            && find_df(&self.fs.master_file, &format!("{file_id:04X}")).is_some()
        {
            return ExecutionResult::Success(HandledCommand {
                responses: vec!["+CRSM: 144,0,6210\r\n".to_string(), "OK\r\n".to_string()],
                action: None,
            });
        }

        // 2. Fallback to default iccprofile sim0 mappings (essential for boot)
        let response_str = match (command, file_id) {
            // 1. EF_DIR (2F00 / 12032)
            (APDU_GET_RESPONSE, 0x2F00) => {
                Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_DIR_FCP))
            }
            (APDU_READ_RECORD, 0x2F00) => {
                if p1 == 1 {
                    Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_DIR_RECORD_CSIM))
                } else if p1 == 2 {
                    Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_DIR_RECORD_USIM))
                } else {
                    Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_DIR_RECORD_EMPTY))
                }
            }
            // 2. EF_ICCID (2FE2 / 12258)
            (APDU_GET_RESPONSE, 0x2FE2) => {
                Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_ICCID_FCP))
            }
            (APDU_READ_BINARY, 0x2FE2) => Some(format!("+CRSM: 144,0,{}\r\n", self.iccid)),
            // 3. EF_AD (6FAD / 28589)
            (APDU_GET_RESPONSE, 0x6FAD) => {
                Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_AD_FCP))
            }
            (APDU_READ_BINARY, 0x6FAD) => {
                Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_AD_DATA_FALLBACK))
            }
            // 4. EF_MSISDN (6F40 / 28480)
            (APDU_GET_RESPONSE, 0x6F40) => {
                Some(format!("+CRSM: 144,0,{}\r\n", iccprofile_for_sim0::EF_MSISDN_FCP))
            }
            (APDU_READ_RECORD, 0x6F40) => {
                Some(format!("+CRSM: 144,0,{}\r\n", self.encode_msisdn()))
            }
            // SELECT (164)
            (APDU_SELECT, _) => Some("+CRSM: 144,0,6210\r\n".to_string()),
            _ => None,
        };

        let resp = response_str.unwrap_or_else(|| "+CRSM: 106,130\r\n".to_string());
        ExecutionResult::Success(HandledCommand {
            responses: vec![resp, "OK\r\n".to_string()],
            action: None,
        })
    }

    fn handle_open_logical_channel(&mut self) -> ExecutionResult {
        if let Some(channel_idx) = self.logical_channels.iter().position(|&open| !open) {
            self.logical_channels[channel_idx] = true;
            ExecutionResult::Success(HandledCommand {
                responses: vec![format!("{}\r\n", channel_idx), "OK\r\n".to_string()],
                action: None,
            })
        } else {
            ExecutionResult::CmeError(CmeError::NoResources)
        }
    }

    fn handle_close_logical_channel(&mut self, channel_id: u8) -> ExecutionResult {
        let idx = channel_id as usize;
        if idx == 0 || idx >= self.logical_channels.len() || !self.logical_channels[idx] {
            return ExecutionResult::CmeError(CmeError::InvalidIndex);
        }
        self.logical_channels[idx] = false;

        // Non-standard: AOSP Goldfish RIL requires "+CCHC" response on channel close to
        // prevent serialization locks.
        // TODO: Extract goldfish-specific quirks into flags.
        ExecutionResult::Success(HandledCommand {
            responses: vec!["+CCHC\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    fn handle_transmit_logical_channel(&self, channel_id: u8, data: &[u8]) -> ExecutionResult {
        let idx = channel_id as usize;
        if idx >= self.logical_channels.len() || !self.logical_channels[idx] {
            return ExecutionResult::CmeError(CmeError::InvalidIndex);
        }

        let data_str = std::str::from_utf8(data).unwrap_or("");
        let data_clean = data_str.trim_matches('"');

        // Check and strip prefix case-insensitively first to avoid double-allocation on
        // heap
        let data_to_match = if data_clean.len() > 2 && data_clean.is_char_boundary(2) {
            let (prefix, remainder) = data_clean.split_at(2);
            if (prefix == "00" || prefix == "01" || prefix == "02" || prefix == "03")
                && (remainder.starts_with("A4")
                    || remainder.starts_with("a4")
                    || remainder.starts_with("C0")
                    || remainder.starts_with("c0")
                    || remainder.starts_with("B0")
                    || remainder.starts_with("b0"))
            {
                remainder
            } else {
                data_clean
            }
        } else {
            data_clean
        };

        let data_uppercase = data_to_match.to_ascii_uppercase();

        let response_data = match data_uppercase.as_str() {
            "A40004025031" => iccprofile_for_sim0::PKCS15_SELECT_RESP,
            "C0000024" => iccprofile_for_sim0::PKCS15_FCP_RESP,
            "B0000010" => iccprofile_for_sim0::PKCS15_READ_RESP,
            "81F2FF0000" => iccprofile_for_sim0::PKCS15_ERROR_RESP,
            _ => {
                warn!(
                    "[SimService] Transmit logical channel: unrecognized APDU payload {:?}, defaulting to dummy success",
                    data_uppercase
                );
                "4,9000"
            }
        };

        ExecutionResult::Success(HandledCommand {
            responses: vec![format!("+CGLA: {}\r\n", response_data), "OK\r\n".to_string()],
            action: None,
        })
    }

    fn handle_change_password(
        &mut self,
        _facility: QuotedString,
        old_password: QuotedString,
        new_password: QuotedString,
    ) -> ExecutionResult {
        if old_password.as_ref() == self.pin1.as_bytes() {
            self.pin1 = String::from_utf8(new_password.0.to_vec()).unwrap();
            ExecutionResult::Success(HandledCommand::ok())
        } else {
            ExecutionResult::Error
        }
    }

    fn handle_query_pin_retries(&self) -> ExecutionResult {
        let retries = self.pin1_retries;
        let response = format!("+SPIC: {retries}\r\n");
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Success(handled)
    }

    fn handle_set_cdma_subscription_source(&mut self, source: u8) -> ExecutionResult {
        self.cdma_subscription_source = source;
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn handle_query_cdma_subscription_source(&self) -> ExecutionResult {
        let source = self.cdma_subscription_source;
        let response = format!("+CCSS: {source}\r\n");
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Success(handled)
    }

    fn handle_set_cdma_roaming_preference(&mut self, preference: u8) -> ExecutionResult {
        self.cdma_roaming_preference = preference;
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn handle_query_cdma_roaming_preference(&self) -> ExecutionResult {
        let preference = self.cdma_roaming_preference;
        let response = format!("+WRMP: {preference}\r\n");
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Success(handled)
    }

    fn handle_sim_authentication(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn handle_update_phone_number(&self, _phone_number: &[u8]) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn encode_msisdn(&self) -> String {
        let msisdn = &self.msisdn;
        if msisdn.is_empty() {
            return iccprofile_for_sim0::EF_MSISDN_RECORD_FALLBACK.to_string();
        }
        let digits: String = msisdn.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return iccprofile_for_sim0::EF_MSISDN_RECORD_FALLBACK.to_string();
        }

        let ton_npi = if msisdn.starts_with('+') || (digits.len() == 11 && digits.starts_with('1'))
        {
            "91"
        } else {
            "81"
        };

        let mut padded_digits = digits.clone();
        if (padded_digits.len() & 1) != 0 {
            padded_digits.push('F');
        }

        let mut swapped = String::new();
        let chars: Vec<char> = padded_digits.chars().collect();
        for chunk in chars.chunks(2) {
            if chunk.len() == 2 {
                swapped.push(chunk[1]);
                swapped.push(chunk[0]);
            }
        }

        let bcd_len = 1 + (padded_digits.len() / 2);
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

    fn is_sim_command(command: &Command) -> bool {
        matches!(
            command,
            Command::GetSimStatus
                | Command::EnterPin(_, _)
                | Command::SimIo { .. }
                | Command::GetImsi
                | Command::GetIccid
                | Command::OpenLogicalChannel(_)
                | Command::CloseLogicalChannel(_)
                | Command::TransmitLogicalChannel(_, _, _)
                | Command::ChangePassword(_, _, _)
                | Command::QueryPinRetries
                | Command::SetCdmaSubscriptionSource(_)
                | Command::QueryCdmaSubscriptionSource
                | Command::SetCdmaRoamingPreference(_)
                | Command::QueryCdmaRoamingPreference
                | Command::SimAuthentication(_)
                | Command::UpdatePhoneNumber(_)
        )
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        if !Self::is_sim_command(command) {
            return ExecutionResult::Unhandled;
        }
        if self.state == SimState::Absent {
            return ExecutionResult::CmeError(CmeError::SimNotInserted); /* SIM not inserted */
        }
        match command {
            Command::GetSimStatus => self.handle_get_sim_status(),
            Command::EnterPin(pin, new_pin) => self.handle_enter_pin(*pin, *new_pin),
            Command::SimIo { command, file_id, p1, p2, p3, data } => {
                self.handle_sim_io(*command, *file_id, *p1, *p2, *p3, *data)
            }
            Command::GetImsi => self.handle_get_imsi(),
            Command::GetIccid => self.handle_get_iccid(),
            Command::OpenLogicalChannel(_) => self.handle_open_logical_channel(),
            Command::CloseLogicalChannel(channel_id) => {
                self.handle_close_logical_channel(*channel_id)
            }
            Command::TransmitLogicalChannel(channel_id, _, data) => {
                self.handle_transmit_logical_channel(*channel_id, data)
            }
            Command::ChangePassword(facility, old_password, new_password) => {
                self.handle_change_password(*facility, *old_password, *new_password)
            }
            Command::QueryPinRetries => self.handle_query_pin_retries(),
            Command::SetCdmaSubscriptionSource(source) => {
                self.handle_set_cdma_subscription_source(*source)
            }
            Command::QueryCdmaSubscriptionSource => self.handle_query_cdma_subscription_source(),
            Command::SetCdmaRoamingPreference(preference) => {
                self.handle_set_cdma_roaming_preference(*preference)
            }
            Command::QueryCdmaRoamingPreference => self.handle_query_cdma_roaming_preference(),
            Command::SimAuthentication(_) => self.handle_sim_authentication(),
            Command::UpdatePhoneNumber(phone_number) => {
                self.handle_update_phone_number(phone_number)
            }
            _ => ExecutionResult::Unhandled,
        }
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
