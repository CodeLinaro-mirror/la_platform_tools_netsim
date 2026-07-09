// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
    parser::{Command, QuotedString},
    sim_service::SimService, // Required for Sim storage
    types::{CommandAction, ExecutionResult, HandledCommand},
};

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

// Holds all state related to the SMS service.
pub struct SmsService {
    // Message reference for the next sent SMS
    message_reference: AtomicU8,
    messages: Vec<Vec<u8>>,
    storage1: MessageStorage,
    storage2: MessageStorage,
    storage3: MessageStorage,
    smsc_address: String,
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

    pub fn handle_sms_body(&mut self, pdu: &[u8]) -> ExecutionResult {
        let action = if self.message_format == MessageFormat::Text {
            let to = self.pending_sms_destination.take().unwrap_or_default();
            let text = std::str::from_utf8(pdu).unwrap_or_default().to_string();
            CommandAction::ReceiveTextSms { to, text }
        } else {
            let processed = crate::pdu::process_outgoing_sms(pdu);
            CommandAction::ReceiveSms { to: processed.to, pdu: processed.pdu }
        };

        let mr = self.message_reference.fetch_add(1, Ordering::Relaxed);
        let response = format!("+CMGS: {mr}\r\n");
        let mut handled = HandledCommand::ok_with_action(action);
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_store_sms(
        &mut self,
        sim_service: &mut SimService,
        pdu: &[u8],
    ) -> ExecutionResult {
        if self.storage1 == MessageStorage::Sim {
            if let Some(index) = sim_service.store_sms(pdu) {
                let response = format!("+CMGW: {index}\r\n");
                let mut handled = HandledCommand::ok();
                handled.responses.insert(0, response);
                ExecutionResult::Handled(handled)
            } else {
                ExecutionResult::Handled(HandledCommand::error())
            }
        } else {
            self.messages.push(pdu.to_vec());
            let response = format!("+CMGW: {}\r\n", self.messages.len());
            let mut handled = HandledCommand::ok();
            handled.responses.insert(0, response);
            ExecutionResult::Handled(handled)
        }
    }

    pub fn handle_delete_sms(
        &mut self,
        sim_service: &mut SimService,
        index: u8,
    ) -> ExecutionResult {
        if self.storage1 == MessageStorage::Sim {
            if sim_service.delete_sms(index) {
                ExecutionResult::Handled(HandledCommand::ok())
            } else {
                ExecutionResult::Handled(HandledCommand::error())
            }
        } else if (index as usize) > 0 && (index as usize - 1) < self.messages.len() {
            self.messages.remove(index as usize - 1);
            ExecutionResult::Handled(HandledCommand::ok())
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_read_sms(&mut self, sim_service: &mut SimService, index: u8) -> ExecutionResult {
        if self.storage1 == MessageStorage::Sim {
            sim_service.read_sms(index)
        } else if let Some(pdu) = index.checked_sub(1).and_then(|i| self.messages.get(i as usize)) {
            let response = format!("+CMGR: 0,,{}\r\n{}\r\n", pdu.len(), hex::encode_upper(pdu));
            let mut handled = HandledCommand::ok();
            handled.responses.insert(0, response);
            ExecutionResult::Handled(handled)
        } else {
            ExecutionResult::Handled(HandledCommand::error())
        }
    }

    pub fn handle_set_sms_message_format(&mut self, format: u8) -> ExecutionResult {
        self.message_format = if format == 1 { MessageFormat::Text } else { MessageFormat::Pdu };
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_preferred_message_storage(
        &mut self,
        storage1: QuotedString,
        storage2: QuotedString,
        storage3: QuotedString,
    ) -> ExecutionResult {
        self.storage1 =
            if storage1.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        self.storage2 =
            if storage2.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        self.storage3 =
            if storage3.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_preferred_message_storage(&self) -> ExecutionResult {
        let response = format!(
            "+CPMS: \"{}\",0,255,\"{}\",0,255,\"{}\",0,255\r\n",
            if self.storage1 == MessageStorage::Sim { "SM" } else { "ME" },
            if self.storage2 == MessageStorage::Sim { "SM" } else { "ME" },
            if self.storage3 == MessageStorage::Sim { "SM" } else { "ME" }
        );
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_send_sms_ack(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_wait_for_store_sms(&mut self, len: u8) -> ExecutionResult {
        self.waiting_for_pdu_len = Some(len as usize);
        self.waiting_for_pdu_store = true;
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["> \r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_cmgs(&mut self, data: &[u8]) -> ExecutionResult {
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
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["> \r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_broadcast_config(
        &mut self,
        mode: u8,
        mids: QuotedString,
        dcss: QuotedString,
    ) -> ExecutionResult {
        self.broadcast_config = (
            mode,
            String::from_utf8(mids.to_vec()).unwrap_or_default(),
            String::from_utf8(dcss.to_vec()).unwrap_or_default(),
        );
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_broadcast_config(&self) -> ExecutionResult {
        let (mode, mids, dcss) = &self.broadcast_config;
        let response = format!("+CSCB: {mode},\"{mids}\",\"{dcss}\"\r\n");
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_set_smsc_address(&mut self, address: QuotedString) -> ExecutionResult {
        self.smsc_address = String::from_utf8(address.to_vec()).unwrap_or_default();
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_get_smsc_address(&self) -> ExecutionResult {
        let response = format!("+CSCA: \"{}\",145\r\n", self.smsc_address);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_remote_sms(&self, pdu: QuotedString) -> ExecutionResult {
        let pdu_bytes = pdu.to_vec();
        let processed = crate::pdu::process_outgoing_sms(&pdu_bytes);
        let action = CommandAction::ReceiveSms { to: processed.to, pdu: processed.pdu };
        ExecutionResult::Handled(HandledCommand::ok_with_action(action))
    }

    // Explicit execute method instead of Trait
    pub fn execute(&mut self, command: &Command, sim_service: &mut SimService) -> ExecutionResult {
        match command {
            Command::SendSms(data) => self.handle_cmgs(data),
            Command::StoreSms(len) => self.handle_wait_for_store_sms(*len),
            Command::ReadSms(index) => self.handle_read_sms(sim_service, *index),
            Command::DeleteSms(index) => self.handle_delete_sms(sim_service, *index),
            Command::SendSmsAck => self.handle_send_sms_ack(),
            Command::SetSmsMessageFormat(format) => self.handle_set_sms_message_format(*format),
            Command::SetPreferredMessageStorage(storage1, storage2, storage3) => {
                self.handle_set_preferred_message_storage(*storage1, *storage2, *storage3)
            }
            Command::QueryPreferredMessageStorage => self.handle_query_preferred_message_storage(),
            Command::BroadcastConfig(mode, mids, dcss) => {
                self.handle_broadcast_config(*mode, *mids, *dcss)
            }
            Command::QueryBroadcastConfig => self.handle_query_broadcast_config(),
            Command::SetSmscAddress(address) => self.handle_set_smsc_address(*address),
            Command::GetSmscAddress => self.handle_get_smsc_address(),
            Command::RemoteSms(pdu) => self.handle_remote_sms(*pdu),
            _ => ExecutionResult::Unhandled,
        }
    }
}
