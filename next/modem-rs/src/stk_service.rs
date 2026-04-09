// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    parser::{Command, QuotedString},
    types::{ExecutionResult, HandledCommand},
};

#[derive(Default)]
pub struct StkService {}

impl StkService {
    // --- Pure command handlers ---

    fn handle_envelope_command(&self, command: &[u8]) -> Vec<u8> {
        // A simple parser for the envelope command.
        // For now, we only care about the command tag.
        if command.len() < 8 {
            return b"ERROR\r\n".to_vec();
        }

        let tag = &command[0..2];
        if tag == b"D1" {
            // Proactive command
            let command_details_tag = &command[6..8];
            if command_details_tag == b"21" {
                // Display Text
                return b"+CUSAT: \"9000\"\r\n".to_vec();
            } else if command_details_tag == b"23" {
                // Get Input
                return b"+CUSAT: \"9000\"\r\n".to_vec();
            }
        } else if tag == b"D3" {
            // Menu Selection
            return b"+CUSATP: \"SubMenu1\"\r\n".to_vec();
        }

        b"ERROR\r\n".to_vec()
    }

    fn handle_set_stk(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_set_stk_enabled(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_set_stk_unsolicited_result(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    fn handle_query_stk_ready(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["+CUSATP: \"SETUP MENU\"\r\n".to_string()],
            action: None,
        })
    }

    fn handle_send_stk_envelope_command(&self, envelope_command: QuotedString) -> ExecutionResult {
        let response = self.handle_envelope_command(envelope_command.as_ref());
        ExecutionResult::Handled(HandledCommand {
            responses: vec![String::from_utf8(response).unwrap_or_default()],
            action: None,
        })
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::QueryStkReady => self.handle_query_stk_ready(),
            Command::SendStkEnvelope(envelope_command) => {
                self.handle_send_stk_envelope_command(*envelope_command)
            }
            Command::SetStk(_) => self.handle_set_stk(),
            Command::SetStkEnabled(_) => self.handle_set_stk_enabled(),
            Command::SetStkUnsolicitedResult(_) => self.handle_set_stk_unsolicited_result(),
            _ => ExecutionResult::Unhandled,
        }
    }
}
