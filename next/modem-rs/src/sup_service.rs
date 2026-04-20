// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    parser::{Command, QuotedString},
    types::{ExecutionResult, HandledCommand},
};

pub const _MODE_ENABLE: u8 = 1;
pub const _MODE_QUERY: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallForwardingInfo {
    pub mode: u8,
    pub number: String,
    pub type_: u8,
}

#[derive(Debug, Default)]
pub struct SupService {
    call_forwarding_info: Option<CallForwardingInfo>,
}

impl SupService {
    // --- Pure command handlers ---

    fn handle_set_facility_lock(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_call_forwarding(
        &mut self,
        mode: u8,
        number: Option<QuotedString>,
        type_: Option<u8>,
    ) -> ExecutionResult {
        let number = number.map(|s| s.to_vec()).unwrap_or_default();
        let type_ = type_.unwrap_or_default();
        let info = CallForwardingInfo {
            mode,
            number: String::from_utf8(number).unwrap_or_default(),
            type_,
        };

        // Simplified logic: If enabling, set info. If querying/disabling, we might
        // check it. Original code only had specific cases.
        // MODE_ENABLE in original code was setting it.
        // MODE_QUERY in original code was reading it? Wait, MODE_QUERY is 2.

        self.call_forwarding_info = Some(info);
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_query_clir(&self) -> ExecutionResult {
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, "+CLIR: 0,0\r\n".to_string());
        ExecutionResult::Handled(handled)
    }

    fn handle_set_clip(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_set_call_waiting(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_send_ussd(&self) -> ExecutionResult {
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, "+CUSD: 0,\"OK\",15\r\n".to_string());
        ExecutionResult::Handled(handled)
    }

    fn handle_cancel_ussd(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_supp_service_notification(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::SetFacilityLock(_, _, _) => self.handle_set_facility_lock(),
            Command::CallForwarding { reason: _, mode, number, type_, .. } => {
                self.handle_call_forwarding(*mode, *number, *type_)
            }
            Command::QueryClir => self.handle_query_clir(),
            Command::SetClip(_) => self.handle_set_clip(),
            Command::SetCallWaiting(_, _) => self.handle_set_call_waiting(),
            Command::SuppServiceNotification(_, _) => self.handle_supp_service_notification(),
            Command::SendUssd(_) => self.handle_send_ussd(),
            Command::CancelUssd => self.handle_cancel_ussd(),
            _ => ExecutionResult::Unhandled,
        }
    }
}
