// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use tracing::debug;

use crate::{
    parser::Command,
    types::{AT_OK, CommandAction, ExecutionResult, HandledCommand, ModemId},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CallDirection {
    Outgoing = 0,
    Incoming = 1,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CallState {
    Active = 0,
    Held = 1,
    Dialing = 2,
    Alerting = 3,
    Incoming = 4,
    Waiting = 5,
}

#[derive(Debug, Clone)]
pub struct CallStatus {
    pub state: CallState,
    pub direction: CallDirection,
    pub is_voice_mode: bool,
    pub is_multi_party: bool,
    pub number: String,
    pub peer_id: Option<ModemId>,
}

// Holds all state related to the call service.
#[derive(Default)]
pub struct CallService {
    pub calls: Vec<CallStatus>,
    mute: bool,
    emergency_mode: bool,
}

impl CallService {
    // --- Helper methods for external services ---

    pub fn receive_hangup(&mut self) {
        self.calls.clear();
    }

    pub fn handle_ring_timeout(&mut self, _call_token: u32) {
        self.calls.retain(|c| c.state != CallState::Alerting);
    }

    pub fn ring(&mut self, number: String) -> ExecutionResult {
        self.calls.push(CallStatus {
            state: CallState::Alerting,
            direction: CallDirection::Incoming,
            is_voice_mode: true,
            is_multi_party: false,
            number,
            peer_id: None,
        });
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["RING\r\n".to_string()],
            action: None,
        })
    }

    pub fn receive_hold(&mut self) {
        debug!("[CallService] Receiving hold");
        if let Some(call) = self.calls.iter_mut().find(|c| c.state == CallState::Active) {
            call.state = CallState::Held;
        }
    }

    pub fn receive_resume(&mut self) {
        debug!("[CallService] Receiving resume");
        if let Some(call) = self.calls.iter_mut().find(|c| c.state == CallState::Held) {
            call.state = CallState::Active;
        }
    }

    pub fn remote_answer(&mut self) -> bool {
        if let Some(call) = self.calls.iter_mut().find(|c| c.state == CallState::Dialing) {
            call.state = CallState::Active;
            return true;
        }
        false
    }

    pub fn is_idle(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn is_dialing(&self) -> bool {
        self.calls.iter().any(|c| c.state == CallState::Dialing)
    }

    pub fn is_alerting(&self) -> bool {
        self.calls.iter().any(|c| c.state == CallState::Alerting)
    }

    pub fn is_active(&self) -> bool {
        self.calls.iter().any(|c| c.state == CallState::Active)
    }

    pub fn is_held(&self) -> bool {
        self.calls.iter().any(|c| c.state == CallState::Held)
    }

    pub fn connect(&mut self, _modem_id: ModemId, peer_id: ModemId) -> Option<Vec<u8>> {
        debug!("[CallService] Connecting to peer {}", peer_id);
        debug!("[CallService] Calls before connect: {:?}", self.calls);
        if let Some(call) = self.calls.iter_mut().find(|c| c.state == CallState::Dialing) {
            call.state = CallState::Active;
            call.peer_id = Some(peer_id);
            return Some(AT_OK.to_vec());
        }
        debug!("[CallService] Calls after connect: {:?}", self.calls);
        None
    }

    // --- Pure command handlers ---

    pub fn handle_dial(&mut self, number: &[u8]) -> ExecutionResult {
        debug!("[CallService] Dialing number: {}", String::from_utf8_lossy(number));
        // GPRS dial commands (e.g. ATD*99#) are GPRS packet-data requests and should
        // fall back to DataService.
        if crate::constants::is_gprs_dial(number) {
            return ExecutionResult::Unhandled;
        }
        if number == b"911" {
            return ExecutionResult::Handled(HandledCommand::ok_with_action(
                CommandAction::InitiateEmergencyCall,
            ));
        }

        debug!("[CallService] Calls before dial: {:?}", self.calls);
        if self.calls.iter().any(|c| c.state == CallState::Dialing) {
            return ExecutionResult::Handled(HandledCommand::default()); // No response, just ignore
        }

        let mut did_hold = false;
        for call in self.calls.iter_mut() {
            if call.state == CallState::Active {
                call.state = CallState::Held;
                did_hold = true;
            }
        }

        let number_str = String::from_utf8(number.to_vec()).unwrap();
        self.calls.push(CallStatus {
            state: CallState::Dialing,
            direction: CallDirection::Outgoing,
            is_voice_mode: true,
            is_multi_party: false,
            number: number_str.clone(),
            peer_id: None,
        });
        debug!("[CallService] Calls after dial: {:?}", self.calls);

        let action = if did_hold {
            CommandAction::InitiateCallAndHold(number_str)
        } else {
            CommandAction::InitiateCall(number_str)
        };

        ExecutionResult::Handled(HandledCommand::ok_with_action(action))
    }

    pub fn handle_answer(&mut self, id: ModemId) -> ExecutionResult {
        debug!("[CallService] Answering call");
        debug!("[CallService] Calls before answer: {:?}", self.calls);
        if let Some(call) = self.calls.iter_mut().find(|c| c.state == CallState::Alerting) {
            call.state = CallState::Active;
            debug!("[CallService] Calls after answer: {:?}", self.calls);
            return ExecutionResult::Handled(HandledCommand::ok_with_action(
                CommandAction::AnswerCall(id),
            ));
        }
        ExecutionResult::Handled(HandledCommand::error())
    }

    pub fn handle_hangup(&mut self, id: ModemId) -> ExecutionResult {
        if self.calls.is_empty() {
            return ExecutionResult::Handled(HandledCommand::error());
        }
        self.calls.clear();
        ExecutionResult::Handled(HandledCommand::ok_with_action(CommandAction::HangupCall(id)))
    }

    pub fn handle_call_hold(&mut self, op: u8) -> ExecutionResult {
        debug!("[CallService] Call hold operation: {}", op);
        debug!("[CallService] Calls before hold op: {:?}", self.calls);
        if op == 2 {
            let active_pos = self.calls.iter().position(|c| c.state == CallState::Active);
            let held_pos = self.calls.iter().position(|c| c.state == CallState::Held);

            if let (Some(ap), Some(hp)) = (active_pos, held_pos) {
                let active_peer = self.calls[ap].peer_id;
                let held_peer = self.calls[hp].peer_id;
                self.calls[ap].state = CallState::Held;
                self.calls[hp].state = CallState::Active;
                debug!("[CallService] Calls after hold op: {:?}", self.calls);
                if let (Some(active_peer), Some(held_peer)) = (active_peer, held_peer) {
                    return ExecutionResult::Handled(HandledCommand::ok_with_action(
                        CommandAction::SwapCalls(active_peer, held_peer),
                    ));
                }
            }
        }
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_current_calls(&self) -> ExecutionResult {
        let mut responses = Vec::new();
        for (i, call) in self.calls.iter().enumerate() {
            let response = format!(
                "+CLCC: {},{},{},{},{},\"{}\",{}\r\n",
                i + 1,
                call.direction as u8,
                call.state as u8,
                if call.is_voice_mode { 0 } else { 1 },
                call.is_multi_party as u8,
                call.number,
                129 // TODO: Handle number type
            );
            debug!("[CallService] Query current calls response: {}", response);
            responses.push(response);
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn handle_remote_call(&mut self, number: &[u8]) -> ExecutionResult {
        let number_str = String::from_utf8(number.to_vec()).unwrap();
        self.calls.push(CallStatus {
            state: CallState::Incoming,
            direction: CallDirection::Incoming,
            is_voice_mode: true,
            is_multi_party: false,
            number: number_str.clone(),
            peer_id: None,
        });
        ExecutionResult::Handled(HandledCommand::ok_with_action(CommandAction::InitiateRemoteCall(
            number_str,
        )))
    }

    pub fn handle_set_mute(&mut self, mute: u8) -> ExecutionResult {
        self.mute = mute == 1;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_mute(&self) -> ExecutionResult {
        let response = format!("+CMUT: {}\r\n", if self.mute { 1 } else { 0 });
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_send_dtmf(&self, _dtmf: &[u8]) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_emergency_mode(&mut self, mode: u8) -> ExecutionResult {
        self.emergency_mode = mode == 1;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_emergency_mode(&self) -> ExecutionResult {
        let response = format!("+WSOS: {}\r\n", if self.emergency_mode { 1 } else { 0 });
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn execute(&mut self, command: &Command, id: ModemId) -> ExecutionResult {
        match command {
            Command::Dial(number) => self.handle_dial(number),
            Command::Answer => self.handle_answer(id),
            Command::Hangup => self.handle_hangup(id),
            Command::CallHold(op) => self.handle_call_hold(*op),
            Command::QueryCurrentCalls => self.handle_query_current_calls(),
            Command::Ring => self.ring("".to_string()),
            Command::RemoteCall(number) => self.handle_remote_call(number),
            Command::SetMute(mute) => self.handle_set_mute(*mute),
            Command::QueryMute => self.handle_query_mute(),
            Command::SendDtmf(dtmf) => self.handle_send_dtmf(dtmf),
            Command::SetEmergencyMode(mode) => self.handle_set_emergency_mode(*mode),
            Command::QueryEmergencyMode => self.handle_query_emergency_mode(),
            _ => ExecutionResult::Unhandled,
        }
    }
}
