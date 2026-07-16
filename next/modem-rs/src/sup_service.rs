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
    clip_enabled: u8,
}

impl SupService {
    // --- Pure command handlers ---

    fn handle_set_facility_lock(&self, _facility: &str, mode: u8) -> ExecutionResult {
        let responses = if mode == 2 {
            vec!["+CLCK: 0\r\n".to_string(), "OK\r\n".to_string()]
        } else {
            vec!["OK\r\n".to_string()]
        };
        ExecutionResult::Success(HandledCommand { responses, action: None })
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
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn handle_query_clir(&self) -> ExecutionResult {
        let responses = vec!["+CLIR: 0,0\r\n".to_string(), "OK\r\n".to_string()];
        ExecutionResult::Success(HandledCommand { responses, action: None })
    }

    fn handle_set_clir(&self, _clir: u8) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn handle_set_clip(&mut self, enabled: u8) -> ExecutionResult {
        self.clip_enabled = enabled;
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn handle_query_clip(&self) -> ExecutionResult {
        let response = format!("+CLIP: {},1\r\n", self.clip_enabled);
        ExecutionResult::Success(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    fn handle_set_call_waiting(
        &self,
        _n: u8,
        mode: Option<u8>,
        class: Option<u8>,
    ) -> ExecutionResult {
        let responses = if let Some(2) = mode {
            let classx = class.unwrap_or(7);
            vec![format!("+CCWA: 0,{}\r\n", classx), "OK\r\n".to_string()]
        } else {
            vec!["OK\r\n".to_string()]
        };
        ExecutionResult::Success(HandledCommand { responses, action: None })
    }

    fn handle_set_ussd(
        &self,
        mode: u8,
        message: Option<QuotedString>,
        _dcs: Option<u8>,
    ) -> ExecutionResult {
        let mut responses = Vec::new();
        if mode == 1 && message.is_some() {
            responses.push("+CUSD: 0,\"OK\",15\r\n".to_string());
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Success(HandledCommand { responses, action: None })
    }

    fn handle_supp_service_notification(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    fn handle_set_colp(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::SetFacilityLock(facility, mode, _, _) => {
                let facility_str = std::str::from_utf8(facility.as_ref()).unwrap_or("");
                self.handle_set_facility_lock(facility_str, *mode)
            }
            Command::CallForwarding { reason: _, mode, number, r#type, .. } => {
                self.handle_call_forwarding(*mode, *number, *r#type)
            }
            Command::QueryClir => self.handle_query_clir(),
            Command::SetClir(clir) | Command::SetClirGoldfish(clir) => self.handle_set_clir(*clir),
            Command::SetClip(enabled) => self.handle_set_clip(*enabled),
            Command::QueryClip => self.handle_query_clip(),
            Command::SetColp(_) => self.handle_set_colp(),
            Command::SetCallWaiting(n, mode, class) => {
                self.handle_set_call_waiting(*n, *mode, *class)
            }
            Command::SuppServiceNotification(_, _) => self.handle_supp_service_notification(),
            Command::SetUssd { mode, message, dcs } => self.handle_set_ussd(*mode, *message, *dcs),
            _ => ExecutionResult::Unhandled,
        }
    }
}
