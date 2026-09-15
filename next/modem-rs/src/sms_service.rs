// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, fmt};

use modem_rs_derive::CommandParser;
use netsim_model::Quirks;
use nom::IResult;

use crate::{
    parser::QuotedString,
    pdu::SubmitPdu,
    sim_service::SimService, // Required for Sim storage
    types::{
        BroadcastConfig, CmsError, CommandAction, ExecutionResult, HandledCommand, Parsable,
        PhoneNumber, Response, SendSmsArgs, SimSmsMessage, SmsAck, SmsBroadcastMode,
        SmsMessageStatus, TypeOfAddress,
    },
};

#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum SmsCommand<'a> {
    /// 3GPP TS 27.005: Send message
    #[command(tag = "AT+CMGS=")]
    SendSms(SendSmsArgs<'a>),
    /// 3GPP TS 27.005: Write message to memory
    #[command(tag = "AT+CMGW=")]
    StoreSms(u16, Option<MessageStatus>),
    /// 3GPP TS 27.005: Read message
    #[command(tag = "AT+CMGR=")]
    ReadSms(usize),
    /// 3GPP TS 27.005: Delete SMS Message
    #[command(tag = "AT+CMGD=")]
    DeleteSms(usize),
    /// 3GPP TS 27.005: New message acknowledgement with value (e.g. AT+CNMA=1)
    #[command(tag = "AT+CNMA=")]
    SendSmsAckWithVal(SmsAck),
    /// 3GPP TS 27.005: New message acknowledgement
    #[command(tag = "AT+CNMA")]
    SendSmsAck,
    /// 3GPP TS 27.005: Set SMS message format
    #[command(tag = "AT+CMGF=")]
    SetSmsMessageFormat(MessageFormat),
    /// 3GPP TS 27.005: Set preferred message storage
    #[command(tag = "AT+CPMS=")]
    SetPreferredMessageStorage(MessageStorage, Option<MessageStorage>, Option<MessageStorage>),
    /// 3GPP TS 27.005: Query preferred message storage
    #[command(tag = "AT+CPMS?")]
    QueryPreferredMessageStorage,
    /// 3GPP TS 27.005: Set broadcast config
    #[command(tag = "AT+CSCB=")]
    BroadcastConfig(SmsBroadcastMode, QuotedString<'a>, QuotedString<'a>),
    /// 3GPP TS 27.005: Query broadcast config
    #[command(tag = "AT+CSCB?")]
    QueryBroadcastConfig,
    /// 3GPP TS 27.005: Set SMSC address
    #[command(tag = "AT+CSCA=")]
    SetSmscAddress(QuotedString<'a>, Option<TypeOfAddress>),
    /// 3GPP TS 27.005: Get SMSC address
    #[command(tag = "AT+CSCA?")]
    GetSmscAddress,
    /// VENDOR: Remote SMS
    #[command(tag = "AT+REMOTESMS=")]
    RemoteSms(QuotedString<'a>),
}

pub use crate::types::MessageStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageStorage {
    Sim,
    #[default]
    Me,
}

impl MessageStorage {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Sim => "SM",
            Self::Me => "ME",
        }
    }
}

impl std::str::FromStr for MessageStorage {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SM" => Ok(Self::Sim),
            "ME" => Ok(Self::Me),
            _ => Err(()),
        }
    }
}

impl fmt::Display for MessageStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> Parsable<'a> for MessageStorage {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, quoted) = QuotedString::parse(input)?;
        quoted.as_str().parse().map(|s| (input, s)).map_err(|_err| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageFormat {
    #[default]
    Pdu,
    Text,
}

impl<'a> Parsable<'a> for MessageFormat {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::Pdu)),
            1 => Ok((input, Self::Text)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmsResponse {
    SendSms {
        message_reference: u8,
    },
    WriteSms {
        index: usize,
    },
    ReadSms {
        status: SmsMessageStatus,
        pdu: Vec<u8>,
    },
    PreferredStorage {
        storage1: MessageStorage,
        storage2: MessageStorage,
        storage3: MessageStorage,
    },
    BroadcastConfig(BroadcastConfig),
    SmscAddress {
        address: Option<PhoneNumber>,
        tosca: TypeOfAddress,
    },
    Prompt {
        goldfish_compat: bool,
    },
}

impl std::fmt::Display for SmsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmsResponse::SendSms { message_reference } => {
                write!(f, "+CMGS: {message_reference}\r\n")
            }
            SmsResponse::WriteSms { index } => write!(f, "+CMGW: {index}\r\n"),
            SmsResponse::ReadSms { status, pdu } => {
                let sca_len = pdu.first().copied().unwrap_or(0) as usize;
                let tpdu_len = pdu.len().saturating_sub(1 + sca_len);
                write!(f, "+CMGR: {status},,{tpdu_len}\r\n{}\r\n", hex::encode_upper(pdu))
            }
            SmsResponse::PreferredStorage { storage1, storage2, storage3 } => {
                write!(
                    f,
                    "+CPMS: \"{storage1}\",0,255,\"{storage2}\",0,255,\"{storage3}\",0,255\r\n"
                )
            }
            SmsResponse::BroadcastConfig(BroadcastConfig { mode, mids, dcss }) => {
                write!(f, "+CSCB: {mode},\"{mids}\",\"{dcss}\"\r\n")
            }
            SmsResponse::SmscAddress { address, tosca } => {
                let addr_str = address.as_ref().map(|a| a.as_str()).unwrap_or("");
                write!(f, "+CSCA: \"{addr_str}\",{tosca}\r\n")
            }
            SmsResponse::Prompt { goldfish_compat } => {
                if *goldfish_compat {
                    write!(f, "> \r")
                } else {
                    write!(f, "> ")
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsSuccess {
    pub response: Option<SmsResponse>,
    pub actions: Vec<CommandAction>,
}

impl SmsSuccess {
    pub fn new(response: Option<SmsResponse>) -> Self {
        Self { response, actions: vec![] }
    }

    pub fn with_actions(response: Option<SmsResponse>, actions: Vec<CommandAction>) -> Self {
        Self { response, actions }
    }
}

type SmsResult = Result<SmsSuccess, ExecutionResult>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SmsTransactionState {
    #[default]
    Idle,
    Sending {
        destination: Option<PhoneNumber>,
        expected_len: usize,
    },
    Storing {
        expected_len: usize,
        status: SmsMessageStatus,
    },
}

impl SmsTransactionState {
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle)
    }
}

const MAX_ME_SMS_CAPACITY: usize = 255;

pub struct SmsService {
    quirks: Quirks,
    message_reference: u8,
    messages: BTreeMap<usize, SimSmsMessage>,
    storage1: MessageStorage,
    storage2: MessageStorage,
    storage3: MessageStorage,
    smsc_address: Option<PhoneNumber>,
    smsc_tosca: TypeOfAddress,
    pub(crate) message_format: MessageFormat,
    pub transaction_state: SmsTransactionState,
    broadcast_config: BroadcastConfig,
}

impl Default for SmsService {
    fn default() -> Self {
        Self {
            message_reference: 1,
            quirks: Default::default(),
            messages: Default::default(),
            storage1: Default::default(),
            storage2: Default::default(),
            storage3: Default::default(),
            smsc_address: Default::default(),
            smsc_tosca: Default::default(),
            message_format: Default::default(),
            transaction_state: Default::default(),
            broadcast_config: Default::default(),
        }
    }
}

impl SmsService {
    pub fn new(quirks: Quirks) -> Self {
        Self { quirks, ..Default::default() }
    }
    // --- Pure command handlers ---

    pub fn get_sms_count(&self) -> usize {
        self.messages.len()
    }

    pub fn next_message_reference(&mut self) -> u8 {
        let mr = self.message_reference;
        self.message_reference = self.message_reference.wrapping_add(1);
        mr
    }

    pub fn is_waiting_for_prompt(&self) -> bool {
        self.transaction_state.is_active()
    }

    fn check_sim_ready(sim_service: &SimService) -> Result<(), CmsError> {
        if !sim_service.is_present() {
            Err(CmsError::SimNotInserted)
        } else if !sim_service.is_ready() {
            Err(CmsError::SimPinRequired)
        } else {
            Ok(())
        }
    }

    fn validate_sim_index(index: usize) -> Result<u8, CmsError> {
        u8::try_from(index).ok().filter(|&i| i > 0).ok_or(CmsError::InvalidMemoryIndex)
    }

    fn handle_send_sms(
        &mut self,
        pdu: &[u8],
        sender: Option<&str>,
        destination: Option<PhoneNumber>,
    ) -> ExecutionResult {
        let message_reference = self.next_message_reference();
        let actions = if self.message_format == MessageFormat::Text {
            let to = destination.map(String::from).unwrap_or_default();
            let text = String::from_utf8_lossy(pdu).into_owned();
            vec![CommandAction::ReceiveTextSms { to, text }]
        } else {
            match self.handle_sms_body(pdu, sender, message_reference) {
                Ok(actions) => actions,
                Err(err) => return err,
            }
        };

        let responses =
            vec![Response::Sms(SmsResponse::SendSms { message_reference }), Response::Ok];
        ExecutionResult::Success(HandledCommand { responses, actions })
    }

    pub fn handle_prompt_input(
        &mut self,
        sim_service: &mut SimService,
        command_bytes: &[u8],
        sender: Option<&str>,
    ) -> Option<ExecutionResult> {
        if !self.transaction_state.is_active() {
            return None;
        }

        // Handle ESC: abort transaction immediately
        if command_bytes.contains(&0x1b) {
            self.transaction_state = SmsTransactionState::Idle;
            return Some(ExecutionResult::Success(HandledCommand::ok()));
        }

        // Look for Ctrl+Z (<Ctrl-Z> / 0x1A) terminator
        let pdu = command_bytes.strip_suffix(b"\x1a")?;

        let exec_res = match std::mem::take(&mut self.transaction_state) {
            SmsTransactionState::Storing { status, .. } => {
                self.handle_store_sms(sim_service, pdu, status).into()
            }
            SmsTransactionState::Sending { destination, .. } => {
                self.handle_send_sms(pdu, sender, destination)
            }
            SmsTransactionState::Idle => unreachable!(),
        };

        Some(exec_res)
    }

    pub fn handle_sms_body(
        &self,
        pdu: &[u8],
        sender: Option<&str>,
        message_reference: u8,
    ) -> Result<Vec<CommandAction>, ExecutionResult> {
        if self.message_format == MessageFormat::Text {
            let text = std::str::from_utf8(pdu).unwrap_or_default().to_string();
            Ok(vec![CommandAction::ReceiveTextSms { to: String::new(), text }])
        } else {
            let raw_pdu = crate::pdu::decode_hex_pdu(pdu)?;
            let _parsed = SubmitPdu::parse(&raw_pdu)
                .map_err(|_err| crate::types::CmsError::InvalidPduParameter)?;
            let processed = crate::pdu::process_outgoing_sms(&raw_pdu, sender, message_reference);
            Ok(vec![CommandAction::ReceiveSms {
                to: processed.to,
                pdu: processed.pdu,
                status_report: processed.status_report,
            }])
        }
    }

    fn allocate_me_slot(&self) -> Option<usize> {
        let mut slot = 1usize;
        for &occupied in self.messages.keys() {
            if occupied == slot {
                slot = slot.checked_add(1)?;
            } else {
                break;
            }
        }
        if slot > MAX_ME_SMS_CAPACITY { None } else { Some(slot) }
    }

    pub fn handle_store_sms(
        &mut self,
        sim_service: &mut SimService,
        pdu: &[u8],
        status: SmsMessageStatus,
    ) -> SmsResult {
        let raw_pdu = crate::pdu::decode_hex_pdu(pdu)?;

        // TS 27.005 §3.2.2: <mem2> (storage2) is for writing and sending messages
        if self.storage2 == MessageStorage::Sim {
            Self::check_sim_ready(sim_service)?;
            let message = SimSmsMessage { status, pdu: raw_pdu };
            if let Some(index) = sim_service.store_sms(message) {
                Ok(SmsSuccess::new(Some(SmsResponse::WriteSms { index: index as usize })))
            } else {
                Err(CmsError::MemoryFull.into())
            }
        } else {
            let index = self.allocate_me_slot().ok_or(CmsError::MemoryFull)?;
            self.messages.insert(index, SimSmsMessage { status, pdu: raw_pdu });
            Ok(SmsSuccess::new(Some(SmsResponse::WriteSms { index })))
        }
    }

    // TS 27.005 §3.2.2: <mem1> (storage1) is for reading and deleting messages
    pub fn handle_delete_sms(&mut self, sim_service: &mut SimService, index: usize) -> SmsResult {
        if self.storage1 == MessageStorage::Sim {
            Self::check_sim_ready(sim_service)?;
            let sim_index = Self::validate_sim_index(index)?;
            if sim_service.delete_sms(sim_index) {
                Ok(SmsSuccess::new(None))
            } else {
                Err(CmsError::InvalidMemoryIndex.into())
            }
        } else if self.messages.remove(&index).is_some() {
            Ok(SmsSuccess::new(None))
        } else {
            Err(CmsError::InvalidMemoryIndex.into())
        }
    }

    // TS 27.005 §3.2.2: <mem1> (storage1) is for reading and deleting messages
    pub fn handle_read_sms(&mut self, sim_service: &mut SimService, index: usize) -> SmsResult {
        if self.storage1 == MessageStorage::Sim {
            Self::check_sim_ready(sim_service)?;
            let sim_index = Self::validate_sim_index(index)?;
            let msg = sim_service
                .read_sms(sim_index)
                .map_err(|_err| CmsError::SimNotInserted)?
                .ok_or(CmsError::InvalidMemoryIndex)?;
            Ok(SmsSuccess::new(Some(SmsResponse::ReadSms { status: msg.status, pdu: msg.pdu })))
        } else {
            let msg = self.messages.get_mut(&index).ok_or(CmsError::InvalidMemoryIndex)?;
            let status = msg.status;
            let pdu = msg.pdu.clone();
            msg.mark_read();
            Ok(SmsSuccess::new(Some(SmsResponse::ReadSms { status, pdu })))
        }
    }

    pub fn handle_set_sms_message_format(&mut self, format: MessageFormat) -> SmsResult {
        self.message_format = format;
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_set_preferred_message_storage(
        &mut self,
        storage1: MessageStorage,
        storage2: Option<MessageStorage>,
        storage3: Option<MessageStorage>,
    ) -> SmsResult {
        self.storage1 = storage1;
        if let Some(s2) = storage2 {
            self.storage2 = s2;
        }
        if let Some(s3) = storage3 {
            self.storage3 = s3;
        }
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_query_preferred_message_storage(&self) -> SmsResult {
        Ok(SmsSuccess::new(Some(SmsResponse::PreferredStorage {
            storage1: self.storage1,
            storage2: self.storage2,
            storage3: self.storage3,
        })))
    }

    pub fn handle_send_sms_ack(&self, ack: SmsAck) -> SmsResult {
        Ok(SmsSuccess::with_actions(None, vec![CommandAction::AcknowledgeIncomingSms { ack }]))
    }

    pub fn handle_wait_for_store_sms(
        &mut self,
        len: u16,
        stat: Option<MessageStatus>,
    ) -> SmsResult {
        let status = stat.unwrap_or(SmsMessageStatus::StoredUnsent);
        self.transaction_state =
            SmsTransactionState::Storing { expected_len: len as usize, status };
        Ok(SmsSuccess::new(Some(SmsResponse::Prompt {
            goldfish_compat: self.quirks.goldfish_ril_37_or_earlier,
        })))
    }

    pub fn handle_wait_for_send_sms(&mut self, args: SendSmsArgs) -> SmsResult {
        match args {
            SendSmsArgs::Text { da, .. } => {
                let destination = da
                    .as_str()
                    .parse::<PhoneNumber>()
                    .map_err(|_err| ExecutionResult::cms_error(CmsError::InvalidPduParameter))?;
                self.transaction_state = SmsTransactionState::Sending {
                    destination: Some(destination),
                    // TODO(b/558794976): Calculate the actual GSM 7-bit packing limit rather
                    // than assuming 160.
                    expected_len: 160,
                };
            }
            SendSmsArgs::Pdu { length } => {
                self.transaction_state =
                    SmsTransactionState::Sending { destination: None, expected_len: length };
            }
        }
        Ok(SmsSuccess::new(Some(SmsResponse::Prompt {
            goldfish_compat: self.quirks.goldfish_ril_37_or_earlier,
        })))
    }

    pub fn handle_broadcast_config(
        &mut self,
        mode: SmsBroadcastMode,
        mids: QuotedString,
        dcss: QuotedString,
    ) -> SmsResult {
        self.broadcast_config = BroadcastConfig {
            mode,
            mids: mids.as_str().to_string(),
            dcss: dcss.as_str().to_string(),
        };
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_query_broadcast_config(&self) -> SmsResult {
        Ok(SmsSuccess::new(Some(SmsResponse::BroadcastConfig(self.broadcast_config.clone()))))
    }

    pub fn handle_set_smsc_address(
        &mut self,
        address: QuotedString,
        tosca: Option<TypeOfAddress>,
    ) -> SmsResult {
        let addr_str = address.as_str();
        if addr_str.is_empty() {
            self.smsc_address = None;
            self.smsc_tosca = tosca.unwrap_or(TypeOfAddress::National);
        } else {
            let phone = addr_str
                .parse::<PhoneNumber>()
                .map_err(|_err| ExecutionResult::cms_error(CmsError::InvalidPduParameter))?;
            self.smsc_tosca = tosca.unwrap_or_else(|| TypeOfAddress::from_number(phone.as_str()));
            self.smsc_address = Some(phone);
        }
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_get_smsc_address(&self) -> SmsResult {
        Ok(SmsSuccess::new(Some(SmsResponse::SmscAddress {
            address: self.smsc_address.clone(),
            tosca: self.smsc_tosca,
        })))
    }

    pub fn handle_remote_sms(&self, pdu: QuotedString) -> SmsResult {
        let pdu_bytes = pdu.as_str().as_bytes();
        let processed = crate::pdu::process_outgoing_sms(pdu_bytes, None, 0);
        let actions = vec![CommandAction::ReceiveSms {
            to: processed.to,
            pdu: processed.pdu,
            status_report: processed.status_report,
        }];
        Ok(SmsSuccess::with_actions(None, actions))
    }

    // Explicit execute method instead of Trait
    pub fn execute<'a>(
        &mut self,
        command: &SmsCommand<'a>,
        sim_service: &mut SimService,
    ) -> ExecutionResult {
        let sms_result = match command {
            SmsCommand::SendSms(args) => self.handle_wait_for_send_sms(*args),
            SmsCommand::StoreSms(len, stat) => self.handle_wait_for_store_sms(*len, *stat),
            SmsCommand::ReadSms(index) => self.handle_read_sms(sim_service, *index),
            SmsCommand::DeleteSms(index) => self.handle_delete_sms(sim_service, *index),
            SmsCommand::SendSmsAck => self.handle_send_sms_ack(SmsAck::Success),
            SmsCommand::SendSmsAckWithVal(ack) => self.handle_send_sms_ack(*ack),
            SmsCommand::SetSmsMessageFormat(format) => self.handle_set_sms_message_format(*format),
            SmsCommand::SetPreferredMessageStorage(storage1, storage2, storage3) => {
                self.handle_set_preferred_message_storage(*storage1, *storage2, *storage3)
            }
            SmsCommand::QueryPreferredMessageStorage => {
                self.handle_query_preferred_message_storage()
            }
            SmsCommand::BroadcastConfig(mode, mids, dcss) => {
                self.handle_broadcast_config(*mode, *mids, *dcss)
            }
            SmsCommand::QueryBroadcastConfig => self.handle_query_broadcast_config(),
            SmsCommand::SetSmscAddress(address, tosca) => {
                self.handle_set_smsc_address(*address, *tosca)
            }
            SmsCommand::GetSmscAddress => self.handle_get_smsc_address(),
            SmsCommand::RemoteSms(pdu) => self.handle_remote_sms(*pdu),
        };
        sms_result.into()
    }
}

impl From<SmsSuccess> for ExecutionResult {
    fn from(success: SmsSuccess) -> Self {
        let mut responses = Vec::new();
        let mut add_ok = true;
        if let Some(resp) = success.response {
            if matches!(resp, SmsResponse::Prompt { .. }) {
                add_ok = false;
            }
            responses.push(resp.into());
        }
        if add_ok {
            responses.push(Response::Ok);
        }
        let actions = success.actions;
        ExecutionResult::Success(HandledCommand { responses, actions })
    }
}
