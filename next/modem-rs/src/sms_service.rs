// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicU8, Ordering};

use modem_rs_derive::CommandParser;
use nom::IResult;

use crate::{
    parser::{QuotedString, parse_raw_data},
    sim_service::SimService, // Required for Sim storage
    types::{
        CommandAction, ExecutionResult, HandledCommand, Parsable, Response, SmsAck,
        SmsBroadcastMode,
    },
};

/// SMS service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum SmsCommand<'a> {
    /// 3GPP TS 27.005: Send message
    #[command(tag = "AT+CMGS=")]
    SendSms(#[parser(parse_raw_data)] &'a [u8]),
    /// 3GPP TS 27.005: Write message to memory
    #[command(tag = "AT+CMGW=")]
    StoreSms(u16, Option<MessageStatus>),
    /// 3GPP TS 27.005: Read message
    #[command(tag = "AT+CMGR=")]
    ReadSms(u8),
    /// 3GPP TS 27.005: Delete SMS Message
    #[command(tag = "AT+CMGD=")]
    DeleteSms(u8),
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
    SetSmscAddress(QuotedString<'a>, Option<u8>),
    /// 3GPP TS 27.005: Get SMSC address
    #[command(tag = "AT+CSCA?")]
    GetSmscAddress,
    /// VENDOR: Remote SMS
    #[command(tag = "AT+REMOTESMS=")]
    RemoteSms(QuotedString<'a>),
}

const TOSCA_INTERNATIONAL: u8 = 145;
const TOSCA_NATIONAL: u8 = 129;

/// 3GPP TS 27.005 § 3.1 / § 3.5.3 message status `<stat>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MessageStatus {
    ReceivedUnread = 0,
    ReceivedRead = 1,
    #[default]
    StoredUnsent = 2,
    StoredSent = 3,
}

impl<'a> Parsable<'a> for MessageStatus {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, val) = u8::parse(input)?;
        match val {
            0 => Ok((input, Self::ReceivedUnread)),
            1 => Ok((input, Self::ReceivedRead)),
            2 => Ok((input, Self::StoredUnsent)),
            3 => Ok((input, Self::StoredSent)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStorage {
    Sim,
    Me,
}

impl<'a> Parsable<'a> for MessageStorage {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, s) = QuotedString::parse(input)?;
        match s.as_ref() {
            b"SM" => Ok((input, Self::Sim)),
            b"ME" => Ok((input, Self::Me)),
            _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageFormat {
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
        mr: u8,
    },
    WriteSms {
        index: usize,
    },
    ReadSms {
        pdu: Vec<u8>,
        stat: MessageStatus,
    },
    PreferredStorage {
        storage1: MessageStorage,
        storage2: MessageStorage,
        storage3: MessageStorage,
    },
    BroadcastConfig {
        mode: SmsBroadcastMode,
        mids: String,
        dcss: String,
    },
    SmscAddress {
        address: String,
        tosca: u8,
    },
    Prompt,
}

impl std::fmt::Display for SmsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmsResponse::SendSms { mr } => write!(f, "+CMGS: {mr}\r\n"),
            SmsResponse::WriteSms { index } => write!(f, "+CMGW: {index}\r\n"),
            SmsResponse::ReadSms { pdu, stat } => {
                let sca_len = pdu.first().copied().unwrap_or(0) as usize;
                let tpdu_len = pdu.len().saturating_sub(1 + sca_len);
                write!(f, "+CMGR: {stat},,{tpdu_len}\r\n{}\r\n", hex::encode_upper(pdu))
            }
            SmsResponse::PreferredStorage { storage1, storage2, storage3 } => {
                let s1 = if *storage1 == MessageStorage::Sim { "SM" } else { "ME" };
                let s2 = if *storage2 == MessageStorage::Sim { "SM" } else { "ME" };
                let s3 = if *storage3 == MessageStorage::Sim { "SM" } else { "ME" };
                write!(f, "+CPMS: \"{s1}\",0,255,\"{s2}\",0,255,\"{s3}\",0,255\r\n")
            }
            SmsResponse::BroadcastConfig { mode, mids, dcss } => {
                write!(f, "+CSCB: {mode},\"{mids}\",\"{dcss}\"\r\n")
            }
            SmsResponse::SmscAddress { address, tosca } => {
                write!(f, "+CSCA: \"{address}\",{tosca}\r\n")
            }
            SmsResponse::Prompt => write!(f, "> "),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSms {
    pub pdu: Vec<u8>,
    pub stat: MessageStatus,
}

// Holds all state related to the SMS service.
pub struct SmsService {
    // Message reference for the next sent SMS
    message_reference: AtomicU8,
    messages: Vec<StoredSms>,
    storage1: MessageStorage,
    storage2: MessageStorage,
    storage3: MessageStorage,
    smsc_address: String,
    smsc_tosca: u8,
    pub(crate) message_format: MessageFormat,
    pending_sms_destination: Option<String>,
    pub waiting_for_pdu: Option<WaitingForPdu>,
    broadcast_config: (SmsBroadcastMode, String, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingForPdu {
    pub len: usize,
    pub store: bool,
    pub stat: Option<MessageStatus>,
}

impl Default for SmsService {
    fn default() -> Self {
        Self {
            message_reference: AtomicU8::new(1),
            messages: Vec::new(),
            storage1: MessageStorage::Me,
            storage2: MessageStorage::Me,
            storage3: MessageStorage::Me,
            smsc_address: "".to_string(),
            smsc_tosca: TOSCA_INTERNATIONAL,
            message_format: MessageFormat::Pdu,
            pending_sms_destination: None,
            waiting_for_pdu: None,
            broadcast_config: (SmsBroadcastMode::Accept, "".to_string(), "".to_string()),
        }
    }
}

impl SmsService {
    // --- Pure command handlers ---

    pub fn get_sms_count(&self) -> usize {
        self.messages.len()
    }

    pub fn handle_send_sms(&mut self, pdu: &[u8], sender: &str) -> SmsResult {
        let mr = self.message_reference.fetch_add(1, Ordering::Relaxed);
        let actions = if self.message_format == MessageFormat::Text {
            let to = self.pending_sms_destination.take().unwrap_or_default();
            let text = std::str::from_utf8(pdu).unwrap_or_default().to_string();
            vec![CommandAction::ReceiveTextSms { to, text }]
        } else {
            let processed = crate::pdu::process_outgoing_sms(pdu, Some(sender), mr);
            vec![CommandAction::ReceiveSms {
                to: processed.to,
                pdu: processed.pdu,
                status_report: processed.status_report,
            }]
        };

        Ok(SmsSuccess::with_actions(Some(SmsResponse::SendSms { mr }), actions))
    }

    pub fn handle_store_sms(
        &mut self,
        sim_service: &mut SimService,
        pdu: &[u8],
        stat: Option<MessageStatus>,
    ) -> SmsResult {
        let stat = stat.unwrap_or_default();
        if self.storage2 == MessageStorage::Sim {
            if let Some(index) = sim_service.store_sms(pdu, stat) {
                Ok(SmsSuccess::new(Some(SmsResponse::WriteSms { index: index as usize })))
            } else {
                Err(ExecutionResult::error())
            }
        } else {
            self.messages.push(StoredSms { pdu: pdu.to_vec(), stat });
            let index = self.messages.len();
            Ok(SmsSuccess::new(Some(SmsResponse::WriteSms { index })))
        }
    }

    pub fn handle_delete_sms(&mut self, sim_service: &mut SimService, index: u8) -> SmsResult {
        if self.storage1 == MessageStorage::Sim {
            if sim_service.delete_sms(index) {
                Ok(SmsSuccess::new(None))
            } else {
                Err(ExecutionResult::error())
            }
        } else if (index as usize) > 0 && (index as usize - 1) < self.messages.len() {
            self.messages.remove(index as usize - 1);
            Ok(SmsSuccess::new(None))
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_read_sms(&mut self, sim_service: &mut SimService, index: u8) -> SmsResult {
        if self.storage1 == MessageStorage::Sim {
            match sim_service.read_sms(index) {
                Ok(Some((pdu, stat))) => {
                    Ok(SmsSuccess::new(Some(SmsResponse::ReadSms { pdu, stat })))
                }
                Ok(None) => Err(ExecutionResult::error()),
                Err(err) => Err(ExecutionResult::cme_error(err)),
            }
        } else if let Some(msg) = index.checked_sub(1).and_then(|i| self.messages.get(i as usize)) {
            Ok(SmsSuccess::new(Some(SmsResponse::ReadSms { pdu: msg.pdu.clone(), stat: msg.stat })))
        } else {
            Err(ExecutionResult::error())
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
        self.waiting_for_pdu = Some(WaitingForPdu { len: len as usize, store: true, stat });
        Ok(SmsSuccess::new(Some(SmsResponse::Prompt)))
    }

    pub fn handle_wait_for_send_sms(&mut self, data: &[u8]) -> SmsResult {
        let len = if self.message_format == MessageFormat::Text {
            let s = String::from_utf8(data.to_vec()).unwrap_or_default();
            let number = s.trim_matches('"').to_string();
            self.pending_sms_destination = Some(number);
            // TODO(b/558794976): Calculate the actual GSM 7-bit packing limit rather
            // than assuming 160.
            160
        } else {
            String::from_utf8(data.to_vec()).unwrap_or_default().parse::<usize>().unwrap_or(0)
        };
        self.waiting_for_pdu = Some(WaitingForPdu { len, store: false, stat: None });
        Ok(SmsSuccess::new(Some(SmsResponse::Prompt)))
    }

    pub fn handle_broadcast_config(
        &mut self,
        mode: SmsBroadcastMode,
        mids: QuotedString,
        dcss: QuotedString,
    ) -> SmsResult {
        self.broadcast_config = (
            mode,
            String::from_utf8(mids.to_vec()).unwrap_or_default(),
            String::from_utf8(dcss.to_vec()).unwrap_or_default(),
        );
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_query_broadcast_config(&self) -> SmsResult {
        let (mode, mids, dcss) = &self.broadcast_config;
        Ok(SmsSuccess::new(Some(SmsResponse::BroadcastConfig {
            mode: *mode,
            mids: mids.clone(),
            dcss: dcss.clone(),
        })))
    }

    pub fn handle_set_smsc_address(
        &mut self,
        address: QuotedString,
        tosca: Option<u8>,
    ) -> SmsResult {
        self.smsc_address = String::from_utf8(address.to_vec()).unwrap_or_default();
        if let Some(t) = tosca {
            self.smsc_tosca = t;
        } else if self.smsc_address.starts_with('+') {
            self.smsc_tosca = TOSCA_INTERNATIONAL;
        } else {
            self.smsc_tosca = TOSCA_NATIONAL;
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
        let pdu_bytes = pdu.to_vec();
        let processed = crate::pdu::process_outgoing_sms(&pdu_bytes, None, 0);
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
            SmsCommand::SendSms(data) => self.handle_wait_for_send_sms(data),
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
            if resp == SmsResponse::Prompt {
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
