use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};

use crate::{
    modem::ModemImpl,
    parser::{Command, QuotedString},
    traits::CommandExecutor,
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
    messages: Mutex<Vec<Vec<u8>>>,
    storage1: Mutex<MessageStorage>,
    storage2: Mutex<MessageStorage>,
    storage3: Mutex<MessageStorage>,
    smsc_address: Mutex<String>,
    message_format: Mutex<MessageFormat>,
    pending_sms_destination: Mutex<Option<String>>,
}

impl SmsService {
    /// Creates a new SmsService.
    pub fn new() -> Self {
        Self {
            message_reference: AtomicU8::new(1),
            messages: Mutex::new(Vec::new()),
            storage1: Mutex::new(MessageStorage::Me),
            storage2: Mutex::new(MessageStorage::Me),
            storage3: Mutex::new(MessageStorage::Me),
            smsc_address: Mutex::new("".to_string()),
            message_format: Mutex::new(MessageFormat::Pdu),
            pending_sms_destination: Mutex::new(None),
        }
    }

    // --- Pure command handlers ---

    pub fn handle_sms_body(&self, _context: &ModemImpl, pdu: &[u8]) -> ExecutionResult {
        dbg!("handle_sms_body");
        let message_format = self.message_format.lock().unwrap();
        dbg!(*message_format);
        let action = if *message_format == MessageFormat::Text {
            let mut dest = self.pending_sms_destination.lock().unwrap();
            let to = dest.take().unwrap_or_default();
            dbg!(&to);
            let text = String::from_utf8(pdu.to_vec()).unwrap_or_default();
            dbg!(&text);
            CommandAction::ReceiveTextSms { to, text }
        } else {
            CommandAction::ReceiveSms(pdu.to_vec())
        };
        dbg!(&action);

        let mr = self.message_reference.fetch_add(1, Ordering::Relaxed);
        let response = format!("+CMGS: {}\r\n", mr);
        let mut handled = HandledCommand::ok_with_action(action);
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_store_sms(&self, context: &ModemImpl, pdu: &[u8]) -> ExecutionResult {
        dbg!("handle_store_sms");
        let storage = self.storage1.lock().unwrap();
        if *storage == MessageStorage::Sim {
            if let Some(index) = context.sim_service.store_sms(pdu) {
                let response = format!("+CMGW: {}\r\n", index);
                let mut handled = HandledCommand::ok();
                handled.responses.insert(0, response);
                ExecutionResult::Handled(handled)
            } else {
                ExecutionResult::Handled(HandledCommand::error())
            }
        } else {
            let mut messages = self.messages.lock().unwrap();
            messages.push(pdu.to_vec());
            let response = format!("+CMGW: {}\r\n", messages.len());
            let mut handled = HandledCommand::ok();
            handled.responses.insert(0, response);
            ExecutionResult::Handled(handled)
        }
    }

    pub fn handle_delete_sms(&self, context: &ModemImpl, index: u8) -> ExecutionResult {
        dbg!("handle_delete_sms");
        let storage = self.storage1.lock().unwrap();
        if *storage == MessageStorage::Sim {
            if context.sim_service.delete_sms(index) {
                ExecutionResult::Handled(HandledCommand::ok())
            } else {
                ExecutionResult::Handled(HandledCommand::error())
            }
        } else {
            let mut messages = self.messages.lock().unwrap();
            if (index as usize - 1) < messages.len() {
                messages.remove(index as usize - 1);
                ExecutionResult::Handled(HandledCommand::ok())
            } else {
                ExecutionResult::Handled(HandledCommand::error())
            }
        }
    }

    pub fn handle_read_sms(&self, context: &ModemImpl, index: u8) -> ExecutionResult {
        dbg!("handle_read_sms");
        let storage = self.storage1.lock().unwrap();
        if *storage == MessageStorage::Sim {
            context.sim_service.read_sms(index)
        } else {
            let messages = self.messages.lock().unwrap();
            if let Some(pdu) = messages.get(index as usize - 1) {
                let response = format!("+CMGR: 0,,{}\r\n{}\r\n", pdu.len(), hex::encode_upper(pdu));
                let mut handled = HandledCommand::ok();
                handled.responses.insert(0, response);
                ExecutionResult::Handled(handled)
            } else {
                ExecutionResult::Handled(HandledCommand::error())
            }
        }
    }

    pub fn handle_set_sms_message_format(&self, format: u8) -> ExecutionResult {
        dbg!("handle_set_sms_message_format");
        let mut message_format = self.message_format.lock().unwrap();
        *message_format = if format == 1 { MessageFormat::Text } else { MessageFormat::Pdu };
        dbg!(*message_format);
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_preferred_message_storage(
        &self,
        storage1: QuotedString,
        storage2: QuotedString,
        storage3: QuotedString,
    ) -> ExecutionResult {
        dbg!("handle_set_preferred_message_storage");
        let mut s1 = self.storage1.lock().unwrap();
        *s1 = if storage1.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        let mut s2 = self.storage2.lock().unwrap();
        *s2 = if storage2.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        let mut s3 = self.storage3.lock().unwrap();
        *s3 = if storage3.as_ref() == b"SM" { MessageStorage::Sim } else { MessageStorage::Me };
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_preferred_message_storage(&self) -> ExecutionResult {
        dbg!("handle_query_preferred_message_storage");
        let s1 = self.storage1.lock().unwrap();
        let s2 = self.storage2.lock().unwrap();
        let s3 = self.storage3.lock().unwrap();
        let response = format!(
            "+CPMS: \"{}\",0,255,\"{}\",0,255,\"{}\",0,255\r\n",
            if *s1 == MessageStorage::Sim { "SM" } else { "ME" },
            if *s2 == MessageStorage::Sim { "SM" } else { "ME" },
            if *s3 == MessageStorage::Sim { "SM" } else { "ME" }
        );
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_send_sms_ack(&self) -> ExecutionResult {
        dbg!("handle_send_sms_ack");
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_wait_for_store_sms(&self, context: &ModemImpl, len: u8) -> ExecutionResult {
        dbg!("handle_wait_for_store_sms");
        context.set_waiting_for_sms_pdu(len as usize, true);
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["> \r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_cmgs(&self, context: &ModemImpl, data: &[u8]) -> ExecutionResult {
        dbg!("handle_cmgs");
        let message_format = self.message_format.lock().unwrap();
        dbg!(*message_format);
        if *message_format == MessageFormat::Text {
            let s = String::from_utf8(data.to_vec()).unwrap_or_default();
            let number = s.trim_matches('"').to_string();
            dbg!(&number);
            let mut dest = self.pending_sms_destination.lock().unwrap();
            *dest = Some(number);
            context.set_waiting_for_sms_pdu(160, false); // Max SMS length
        } else {
            let len =
                String::from_utf8(data.to_vec()).unwrap_or_default().parse::<usize>().unwrap_or(0);
            dbg!(len);
            context.set_waiting_for_sms_pdu(len, false);
        }
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["> \r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_broadcast_config(
        &self,
        _mode: u8,
        _mids: QuotedString,
        _dcss: QuotedString,
    ) -> ExecutionResult {
        dbg!("handle_broadcast_config");
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_smsc_address(&self, address: QuotedString) -> ExecutionResult {
        dbg!("handle_set_smsc_address");
        let mut smsc_address = self.smsc_address.lock().unwrap();
        *smsc_address = String::from_utf8(address.to_vec()).unwrap_or_default();
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_get_smsc_address(&self) -> ExecutionResult {
        dbg!("handle_get_smsc_address");
        let smsc_address = self.smsc_address.lock().unwrap();
        let response = format!("+CSCA: \"{}\",145\r\n", smsc_address);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_remote_sms(&self, pdu: QuotedString) -> ExecutionResult {
        dbg!("handle_remote_sms");
        let action = CommandAction::ReceiveSms(pdu.to_vec());
        ExecutionResult::Handled(HandledCommand::ok_with_action(action))
    }
}

impl CommandExecutor for SmsService {
    fn execute(&self, context: &ModemImpl, command: &Command) -> ExecutionResult {
        dbg!(command);
        match command {
            Command::SendSms(data) => self.handle_cmgs(context, data),
            Command::StoreSms(len) => self.handle_wait_for_store_sms(context, *len),
            Command::ReadSms(index) => self.handle_read_sms(context, *index),
            Command::DeleteSms(index) => self.handle_delete_sms(context, *index),
            Command::SendSmsAck => self.handle_send_sms_ack(),
            Command::SetSmsMessageFormat(format) => self.handle_set_sms_message_format(*format),
            Command::SetPreferredMessageStorage(storage1, storage2, storage3) => {
                self.handle_set_preferred_message_storage(*storage1, *storage2, *storage3)
            }
            Command::QueryPreferredMessageStorage => self.handle_query_preferred_message_storage(),
            Command::BroadcastConfig(mode, mids, dcss) => {
                self.handle_broadcast_config(*mode, *mids, *dcss)
            }
            Command::SetSmscAddress(address) => self.handle_set_smsc_address(*address),
            Command::GetSmscAddress => self.handle_get_smsc_address(),
            Command::RemoteSms(pdu) => self.handle_remote_sms(*pdu),
            _ => ExecutionResult::Unhandled,
        }
    }
}
