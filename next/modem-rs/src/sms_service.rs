// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicU8, Ordering};

use modem_rs_derive::CommandParser;

use crate::{
    parser::{QuotedString, parse_raw_data},
    sim_service::SimService, // Required for Sim storage
    types::{CommandAction, ExecutionResult, HandledCommand, Parsable, Response},
};

/// SMS service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum SmsCommand<'a> {
    /// 3GPP TS 27.005: Send message
    #[command(tag = "AT+CMGS=")]
    SendSms(#[parser(parse_raw_data)] &'a [u8]),
    /// 3GPP TS 27.005: Write message to memory
    #[command(tag = "AT+CMGW=")]
    StoreSms(u8),
    /// 3GPP TS 27.005: Read message
    #[command(tag = "AT+CMGR=")]
    ReadSms(u8),
    /// 3GPP TS 27.005: Delete SMS Message
    #[command(tag = "AT+CMGD=")]
    DeleteSms(u8),
    /// 3GPP TS 27.005: New message acknowledgement with value (e.g. AT+CNMA=1)
    #[command(tag = "AT+CNMA=")]
    SendSmsAckWithVal(u8),
    /// 3GPP TS 27.005: New message acknowledgement
    #[command(tag = "AT+CNMA")]
    SendSmsAck,
    /// 3GPP TS 27.005: Set SMS message format
    #[command(tag = "AT+CMGF=")]
    SetSmsMessageFormat(u8),
    /// 3GPP TS 27.005: Set preferred message storage
    #[command(tag = "AT+CPMS=")]
    SetPreferredMessageStorage(QuotedString<'a>, QuotedString<'a>, QuotedString<'a>),
    /// 3GPP TS 27.005: Query preferred message storage
    #[command(tag = "AT+CPMS?")]
    QueryPreferredMessageStorage,
    /// 3GPP TS 27.005: Set broadcast config
    #[command(tag = "AT+CSCB=")]
    BroadcastConfig(u8, QuotedString<'a>, QuotedString<'a>),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStorage {
    Sim,
    Me,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageFormat {
    Pdu,
    Text,
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
    },
    PreferredStorage {
        storage1: MessageStorage,
        storage2: MessageStorage,
        storage3: MessageStorage,
    },
    BroadcastConfig {
        mode: u8,
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
            SmsResponse::ReadSms { pdu } => {
                write!(f, "+CMGR: 0,,{}\r\n{}\r\n", pdu.len(), hex::encode_upper(pdu))
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
            SmsResponse::Prompt => write!(f, "> \r\n"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsSuccess {
    pub response: Option<SmsResponse>,
    pub action: CommandAction,
}

impl SmsSuccess {
    pub fn new(response: Option<SmsResponse>) -> Self {
        Self { response, action: CommandAction::None }
    }

    pub fn with_action(response: Option<SmsResponse>, action: CommandAction) -> Self {
        Self { response, action }
    }
}

type SmsResult = Result<SmsSuccess, ExecutionResult>;

// Holds all state related to the SMS service.
pub struct SmsService {
    // Message reference for the next sent SMS
    message_reference: AtomicU8,
    messages: Vec<Vec<u8>>,
    storage1: MessageStorage,
    storage2: MessageStorage,
    storage3: MessageStorage,
    smsc_address: String,
    smsc_tosca: u8,
    pub(crate) message_format: MessageFormat,
    pending_sms_destination: Option<String>,
    pub waiting_for_pdu_len: Option<usize>,
    pub waiting_for_pdu_store: bool,
    broadcast_config: (u8, String, String),
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
            waiting_for_pdu_len: None,
            waiting_for_pdu_store: false,
            broadcast_config: (0, "".to_string(), "".to_string()),
        }
    }
}

impl SmsService {
    // --- Pure command handlers ---

    pub fn get_sms_count(&self) -> usize {
        self.messages.len()
    }

    pub fn handle_sms_body(&mut self, pdu: &[u8]) -> SmsResult {
        let action = if self.message_format == MessageFormat::Text {
            let to = self.pending_sms_destination.take().unwrap_or_default();
            let text = std::str::from_utf8(pdu).unwrap_or_default().to_string();
            CommandAction::ReceiveTextSms { to, text }
        } else {
            let processed = crate::pdu::process_outgoing_sms(pdu);
            CommandAction::ReceiveSms { to: processed.to, pdu: processed.pdu }
        };

        let mr = self.message_reference.fetch_add(1, Ordering::Relaxed);
        Ok(SmsSuccess::with_action(Some(SmsResponse::SendSms { mr }), action))
    }

    pub fn handle_store_sms(&mut self, sim_service: &mut SimService, pdu: &[u8]) -> SmsResult {
        if self.storage1 == MessageStorage::Sim {
            if let Some(index) = sim_service.store_sms(pdu) {
                Ok(SmsSuccess::new(Some(SmsResponse::WriteSms { index: index as usize })))
            } else {
                Err(ExecutionResult::error())
            }
        } else {
            self.messages.push(pdu.to_vec());
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
                Ok(Some(pdu)) => Ok(SmsSuccess::new(Some(SmsResponse::ReadSms { pdu }))),
                Ok(None) => Err(ExecutionResult::error()),
                Err(err) => Err(ExecutionResult::cme_error(err)),
            }
        } else if let Some(pdu) = index.checked_sub(1).and_then(|i| self.messages.get(i as usize)) {
            Ok(SmsSuccess::new(Some(SmsResponse::ReadSms { pdu: pdu.clone() })))
        } else {
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_set_sms_message_format(&mut self, format: u8) -> SmsResult {
        self.message_format = if format == 1 { MessageFormat::Text } else { MessageFormat::Pdu };
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_set_preferred_message_storage(
        &mut self,
        storage1: QuotedString,
        storage2: QuotedString,
        storage3: QuotedString,
    ) -> SmsResult {
        self.storage1 =
            if storage1.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        self.storage2 =
            if storage2.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        self.storage3 =
            if storage3.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_query_preferred_message_storage(&self) -> SmsResult {
        Ok(SmsSuccess::new(Some(SmsResponse::PreferredStorage {
            storage1: self.storage1,
            storage2: self.storage2,
            storage3: self.storage3,
        })))
    }

    pub fn handle_send_sms_ack(&self) -> SmsResult {
        Ok(SmsSuccess::new(None))
    }

    pub fn handle_wait_for_store_sms(&mut self, len: u8) -> SmsResult {
        self.waiting_for_pdu_len = Some(len as usize);
        self.waiting_for_pdu_store = true;
        Ok(SmsSuccess::new(Some(SmsResponse::Prompt)))
    }

    pub fn handle_cmgs(&mut self, data: &[u8]) -> SmsResult {
        if self.message_format == MessageFormat::Text {
            let s = String::from_utf8(data.to_vec()).unwrap_or_default();
            let number = s.trim_matches('"').to_string();
            self.pending_sms_destination = Some(number);
            self.waiting_for_pdu_len = Some(160);
            self.waiting_for_pdu_store = false;
        } else {
            let len =
                String::from_utf8(data.to_vec()).unwrap_or_default().parse::<usize>().unwrap_or(0);
            self.waiting_for_pdu_len = Some(len);
            self.waiting_for_pdu_store = false;
        }
        Ok(SmsSuccess::new(Some(SmsResponse::Prompt)))
    }

    pub fn handle_broadcast_config(
        &mut self,
        mode: u8,
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
        let processed = crate::pdu::process_outgoing_sms(&pdu_bytes);
        let action = CommandAction::ReceiveSms { to: processed.to, pdu: processed.pdu };
        Ok(SmsSuccess::with_action(None, action))
    }

    // Explicit execute method instead of Trait
    pub fn execute<'a>(
        &mut self,
        command: &SmsCommand<'a>,
        sim_service: &mut SimService,
    ) -> ExecutionResult {
        let sms_result = match command {
            SmsCommand::SendSms(data) => self.handle_cmgs(data),
            SmsCommand::StoreSms(len) => self.handle_wait_for_store_sms(*len),
            SmsCommand::ReadSms(index) => self.handle_read_sms(sim_service, *index),
            SmsCommand::DeleteSms(index) => self.handle_delete_sms(sim_service, *index),
            SmsCommand::SendSmsAck | SmsCommand::SendSmsAckWithVal(_) => self.handle_send_sms_ack(),
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
        let action =
            if success.action != CommandAction::None { Some(success.action) } else { None };
        ExecutionResult::Success(HandledCommand { responses, action })
    }
}
