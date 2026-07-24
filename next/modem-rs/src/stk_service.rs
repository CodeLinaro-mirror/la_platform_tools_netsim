// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    parser::{Command, QuotedString},
    types::{ExecutionResult, HandledCommand},
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum StkResponse {
    StkReady { ready: u8, support: u8 },
    UsatEnvelopeResponse(String),
    UsatProactiveCommand(String),
}

impl std::fmt::Display for StkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StkResponse::StkReady { ready, support } => {
                write!(f, "+CUSATD: {ready}, {support}\r\n")
            }
            StkResponse::UsatEnvelopeResponse(val) => {
                write!(f, "+CUSATE: {val}\r\n")
            }
            StkResponse::UsatProactiveCommand(val) => {
                write!(f, "+CUSATP: \"{val}\"\r\n")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StkError {
    Error,
}

#[derive(Default)]
struct StkExecutionResult {
    response: Option<StkResponse>,
    urcs: Vec<StkResponse>,
}

type StkResult = Result<StkExecutionResult, StkError>;

impl From<StkResult> for ExecutionResult {
    fn from(res: StkResult) -> Self {
        match res {
            Ok(stk_res) => {
                let mut handled = HandledCommand::ok();
                if let Some(resp) = stk_res.response {
                    let resp_str = resp.to_string();
                    if !resp_str.is_empty() {
                        handled.responses.insert(0, resp_str);
                    }
                }
                for urc in stk_res.urcs {
                    let urc_str = urc.to_string();
                    if !urc_str.is_empty() {
                        handled.responses.push(urc_str);
                    }
                }
                ExecutionResult::Success(handled)
            }
            Err(StkError::Error) => ExecutionResult::Error,
        }
    }
}

#[derive(Default)]
pub struct StkService {}

impl StkService {
    // --- Pure command handlers ---

    fn handle_envelope_command(&self, command: &[u8]) -> StkResult {
        // A simple parser for the envelope command.
        // For now, we only care about the command tag.
        if command.len() < 8 {
            return Err(StkError::Error);
        }

        let tag = &command[0..2];
        let mut urcs = Vec::new();
        if tag == b"D1" {
            // Proactive command
            let command_details_tag = &command[6..8];
            if command_details_tag == b"21" {
                // Display Text
                urcs.push(StkResponse::UsatProactiveCommand("9000".to_string()));
            } else if command_details_tag == b"23" {
                // Get Input
                urcs.push(StkResponse::UsatProactiveCommand("9000".to_string()));
            }
        } else if tag == b"D3" {
            // Menu Selection
            urcs.push(StkResponse::UsatProactiveCommand("SubMenu1".to_string()));
        } else {
            return Err(StkError::Error);
        }

        Ok(StkExecutionResult {
            response: Some(StkResponse::UsatEnvelopeResponse("0".to_string())),
            urcs,
        })
    }

    fn handle_set_stk(&self) -> StkResult {
        Ok(StkExecutionResult::default())
    }

    fn handle_set_stk_enabled(&self) -> StkResult {
        Ok(StkExecutionResult::default())
    }

    fn handle_set_stk_unsolicited_result(&self) -> StkResult {
        Ok(StkExecutionResult::default())
    }

    fn handle_query_stk_ready(&self) -> StkResult {
        Ok(StkExecutionResult {
            response: Some(StkResponse::StkReady { ready: 1, support: 1 }),
            urcs: Vec::new(),
        })
    }

    fn handle_send_stk_envelope_command(&self, envelope_command: QuotedString) -> StkResult {
        self.handle_envelope_command(envelope_command.as_ref())
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::QueryStkReady => self.handle_query_stk_ready().into(),
            Command::SendStkEnvelope(envelope_command) => {
                self.handle_send_stk_envelope_command(*envelope_command).into()
            }
            Command::SetStk(_) => self.handle_set_stk().into(),
            Command::SetStkEnabled(_) => self.handle_set_stk_enabled().into(),
            Command::SetStkUnsolicitedResult(_) => self.handle_set_stk_unsolicited_result().into(),
            _ => ExecutionResult::Unhandled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExecutionResult;

    #[test]
    fn test_stk_result_to_execution_result() {
        // Test Ok with response and URCs (AT+CUSATE case)
        let res: StkResult = Ok(StkExecutionResult {
            response: Some(StkResponse::UsatEnvelopeResponse("0".to_string())),
            urcs: vec![StkResponse::UsatProactiveCommand("9000".to_string())],
        });
        let exec_res: ExecutionResult = res.into();
        if let ExecutionResult::Success(handled) = exec_res {
            assert_eq!(
                handled.responses,
                vec![
                    "+CUSATE: 0\r\n".to_string(),
                    "OK\r\n".to_string(),
                    "+CUSATP: \"9000\"\r\n".to_string()
                ]
            );
        } else {
            panic!("Expected Success");
        }

        // Test Ok with only response (AT+CUSATD? case)
        let res: StkResult = Ok(StkExecutionResult {
            response: Some(StkResponse::StkReady { ready: 1, support: 1 }),
            urcs: Vec::new(),
        });
        let exec_res: ExecutionResult = res.into();
        if let ExecutionResult::Success(handled) = exec_res {
            assert_eq!(
                handled.responses,
                vec!["+CUSATD: 1, 1\r\n".to_string(), "OK\r\n".to_string()]
            );
        } else {
            panic!("Expected Success");
        }

        // Test Ok with empty result (AT+STK=1 case)
        let res: StkResult = Ok(StkExecutionResult::default());
        let exec_res: ExecutionResult = res.into();
        if let ExecutionResult::Success(handled) = exec_res {
            assert_eq!(handled.responses, vec!["OK\r\n".to_string()]);
        } else {
            panic!("Expected Success");
        }

        // Test Err
        let res: StkResult = Err(StkError::Error);
        let exec_res: ExecutionResult = res.into();
        assert!(matches!(exec_res, ExecutionResult::Error));
    }
}
