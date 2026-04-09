// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::{
    config::{DedicatedFile, ElementaryFile, FileSystem, SimFile, SimProfile},
    parser::{Command, QuotedString},
    types::{ExecutionResult, HandledCommand, AT_ERROR, AT_OK, DEFAULT_PIN},
};

const DEFAULT_PUK: &str = "12345678";

// Represents the state of the SIM card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimState {
    Ready,
    PinRequired,
    PukRequired,
}

// Holds all state related to the SIM card.
pub struct SimService {
    state: SimState,
    imsi: String,
    iccid: String,
    pin1: String,
    pin1_retries: u8,
    puk1_retries: u8,
    fs: FileSystem,
    sms_messages: HashMap<u8, Vec<u8>>,
    logical_channels: u8,
    cdma_subscription_source: u8,
    cdma_roaming_preference: u8,
}

impl SimService {
    /// Creates a new SimService from a SIM profile configuration.
    pub fn new(profile: &SimProfile) -> Self {
        let state = match profile.pin_profile.state.as_str() {
            "EnabledNotVerified" => SimState::PinRequired,
            _ => SimState::Ready,
        };

        Self {
            state,
            imsi: profile.imsi.clone(),
            iccid: profile.iccid.clone(),
            pin1: DEFAULT_PIN.to_string(),
            pin1_retries: 3,
            puk1_retries: 10,
            fs: profile.sim_io.file_system.clone(),
            sms_messages: HashMap::new(),
            logical_channels: 0,
            cdma_subscription_source: 0,
            cdma_roaming_preference: 0,
        }
    }

    // --- Public API for other services ---

    pub fn get_sms_count(&self) -> usize {
        self.sms_messages.len()
    }

    pub fn store_sms(&mut self, pdu: &[u8]) -> Option<u8> {
        let index = self.sms_messages.len() as u8 + 1;
        self.sms_messages.insert(index, pdu.to_vec());
        Some(index)
    }

    pub fn read_sms(&self, index: u8) -> ExecutionResult {
        if let Some(pdu) = self.sms_messages.get(&index) {
            let response = format!("+CMGR: 0,,{}\r\n{}\r\n", pdu.len(), hex::encode_upper(pdu));
            let mut handled = HandledCommand::ok();
            handled.responses.insert(0, response);
            ExecutionResult::Handled(handled)
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn delete_sms(&mut self, index: u8) -> bool {
        self.sms_messages.remove(&index).is_some()
    }

    // --- Pure command handlers ---

    fn handle_get_sim_status(&self) -> ExecutionResult {
        let response_str = match self.state {
            SimState::Ready => "+CPIN: READY\r\n",
            SimState::PinRequired => "+CPIN: SIM PIN\r\n",
            SimState::PukRequired => "+CPIN: SIM PUK\r\n",
        };
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response_str.to_string());
        ExecutionResult::Handled(handled)
    }

    fn handle_enter_pin(
        &mut self,
        pin_or_puk: QuotedString,
        new_pin: Option<QuotedString>,
    ) -> ExecutionResult {
        let response = match self.state {
            SimState::Ready => {
                if pin_or_puk.as_ref() == self.pin1.as_bytes() {
                    AT_OK.to_vec()
                } else {
                    self.pin1_retries -= 1;
                    if self.pin1_retries == 0 {
                        self.state = SimState::PukRequired;
                    }
                    AT_ERROR.to_vec()
                }
            }
            SimState::PinRequired => {
                if pin_or_puk.as_ref() == self.pin1.as_bytes() {
                    self.state = SimState::Ready;
                    AT_OK.to_vec()
                } else {
                    self.pin1_retries -= 1;
                    if self.pin1_retries == 0 {
                        self.state = SimState::PukRequired;
                    }
                    AT_ERROR.to_vec()
                }
            }
            SimState::PukRequired => {
                if let Some(new_pin) = new_pin {
                    if pin_or_puk.as_ref() == DEFAULT_PUK.as_bytes() {
                        let _ = new_pin;
                        self.state = SimState::Ready;
                        self.pin1_retries = 3;
                        self.puk1_retries = 10;
                        AT_OK.to_vec()
                    } else {
                        self.puk1_retries -= 1;
                        AT_ERROR.to_vec()
                    }
                } else {
                    AT_ERROR.to_vec()
                }
            }
        };
        ExecutionResult::Handled(HandledCommand {
            responses: vec![String::from_utf8(response).unwrap_or_default()],
            action: None,
        })
    }

    fn handle_get_imsi(&self) -> ExecutionResult {
        let response = format!("{}\r\n", self.imsi);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    fn handle_get_iccid(&self) -> ExecutionResult {
        let response = format!("{}\r\n", self.iccid);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    fn handle_sim_io(
        &self,
        command: u16,
        file_id: u16,
        _data: Option<QuotedString>,
    ) -> ExecutionResult {
        match command {
            176 => {
                // READ BINARY
                if let Some(ef) = find_ef(&self.fs.master_file, &format!("{:04X}", file_id)) {
                    let mut response = b"+CRSM: 144,0,\"".to_vec();
                    response.extend_from_slice(ef.data.as_bytes());
                    response.extend_from_slice(b"\"\r\n");
                    let mut handled = HandledCommand::ok();
                    handled.responses.insert(0, String::from_utf8(response).unwrap());
                    ExecutionResult::Handled(handled)
                } else {
                    ExecutionResult::Handled(HandledCommand::error())
                }
            }
            162 => {
                // SELECT
                if find_df(&self.fs.master_file, &format!("{:04X}", file_id)).is_some() {
                    let mut handled = HandledCommand::ok();
                    handled.responses.insert(0, "+CRSM: 144,0,\"6210\"\r\n".to_string());
                    ExecutionResult::Handled(handled)
                } else {
                    ExecutionResult::Handled(HandledCommand::error())
                }
            }
            _ => ExecutionResult::Handled(HandledCommand::error()),
        }
    }

    fn handle_open_logical_channel(&mut self) -> ExecutionResult {
        self.logical_channels += 1;
        let response = format!("+CCHO: {}\r\n", self.logical_channels);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    fn handle_close_logical_channel(&mut self, channel_id: u8) -> ExecutionResult {
        if channel_id > self.logical_channels {
            return ExecutionResult::Handled(HandledCommand::error());
        }
        self.logical_channels -= 1;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_transmit_logical_channel(&self, channel_id: u8, _data: &[u8]) -> ExecutionResult {
        if channel_id > self.logical_channels {
            return ExecutionResult::Handled(HandledCommand::error());
        }

        let response = "+CGLA: 10, \"9000\"\r\n".to_string();
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    fn handle_change_password(
        &mut self,
        _facility: QuotedString,
        old_password: QuotedString,
        new_password: QuotedString,
    ) -> ExecutionResult {
        if old_password.as_ref() == self.pin1.as_bytes() {
            self.pin1 = String::from_utf8(new_password.0.to_vec()).unwrap();
            ExecutionResult::Handled(HandledCommand::ok())
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    fn handle_query_pin_retries(&self) -> ExecutionResult {
        let retries = self.pin1_retries;
        let response = format!("+SPIC: {}\r\n", retries);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    fn handle_set_cdma_subscription_source(&mut self, source: u8) -> ExecutionResult {
        self.cdma_subscription_source = source;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_query_cdma_subscription_source(&self) -> ExecutionResult {
        let source = self.cdma_subscription_source;
        let response = format!("+CCSS: {}\r\n", source);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    fn handle_set_cdma_roaming_preference(&mut self, preference: u8) -> ExecutionResult {
        self.cdma_roaming_preference = preference;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_query_cdma_roaming_preference(&self) -> ExecutionResult {
        let preference = self.cdma_roaming_preference;
        let response = format!("+WRMP: {}\r\n", preference);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    fn handle_sim_authentication(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_update_phone_number(&self, _phone_number: &[u8]) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::GetSimStatus => self.handle_get_sim_status(),
            Command::EnterPin(pin, new_pin) => self.handle_enter_pin(*pin, *new_pin),
            Command::SimIo { command, file_id, data, .. } => {
                self.handle_sim_io(*command, *file_id, *data)
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
        if let SimFile::Df(df) = file {
            if let Some(found) = find_df(df, id) {
                return Some(found);
            }
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
