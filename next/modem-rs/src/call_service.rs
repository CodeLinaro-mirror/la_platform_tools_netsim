// src/call_service.rs

use std::sync::{Arc, Mutex};

use crate::{
    modem::ModemImpl,
    parser::Command,
    traits::CommandExecutor,
    types::{CallbacksExt, CommandAction, ExecutionResult, HandledCommand, ModemId},
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
pub struct CallService {
    pub calls: Mutex<Vec<CallStatus>>,
    mute: Mutex<bool>,
    emergency_mode: Mutex<bool>,
}

impl CallService {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            mute: Mutex::new(false),
            emergency_mode: Mutex::new(false),
        }
    }

    // --- Helper methods for external services ---

    pub fn receive_hangup(&self) {
        self.calls.lock().unwrap().clear();
    }

    pub fn handle_ring_timeout(&self, _call_token: u32) {
        let mut calls = self.calls.lock().unwrap();
        calls.retain(|c| c.state != CallState::Alerting);
    }

    pub fn ring(&self, number: String) -> ExecutionResult {
        let mut calls = self.calls.lock().unwrap();
        calls.push(CallStatus {
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

    pub fn receive_hold(&self) {
        log::debug!("[CallService] Receiving hold");
        let mut calls = self.calls.lock().unwrap();
        if let Some(call) = calls.iter_mut().find(|c| c.state == CallState::Active) {
            call.state = CallState::Held;
        }
    }

    pub fn receive_resume(&self) {
        log::debug!("[CallService] Receiving resume");
        let mut calls = self.calls.lock().unwrap();
        if let Some(call) = calls.iter_mut().find(|c| c.state == CallState::Held) {
            call.state = CallState::Active;
        }
    }

    pub fn is_idle(&self) -> bool {
        self.calls.lock().unwrap().is_empty()
    }

    pub fn is_dialing(&self) -> bool {
        self.calls.lock().unwrap().iter().any(|c| c.state == CallState::Dialing)
    }

    pub fn is_alerting(&self) -> bool {
        self.calls.lock().unwrap().iter().any(|c| c.state == CallState::Alerting)
    }

    pub fn is_active(&self) -> bool {
        self.calls.lock().unwrap().iter().any(|c| c.state == CallState::Active)
    }

    pub fn is_held(&self) -> bool {
        self.calls.lock().unwrap().iter().any(|c| c.state == CallState::Held)
    }

    pub fn connect(
        &self,
        callbacks: &Arc<dyn crate::types::Callbacks>,
        modem_id: ModemId,
        peer_id: ModemId,
    ) {
        log::debug!("[CallService] Connecting to peer {}", peer_id);
        let mut calls = self.calls.lock().unwrap();
        log::debug!("[CallService] Calls before connect: {:?}", calls);
        if let Some(call) = calls.iter_mut().find(|c| c.state == CallState::Dialing) {
            call.state = CallState::Active;
            call.peer_id = Some(peer_id);
            callbacks.send_ok(modem_id);
        }
        log::debug!("[CallService] Calls after connect: {:?}", calls);
    }

    // --- Pure command handlers ---

    pub fn handle_dial(&self, number: &[u8]) -> ExecutionResult {
        log::debug!("[CallService] Dialing number: {}", String::from_utf8_lossy(number));
        if number == b"911" {
            return ExecutionResult::Handled(HandledCommand::ok_with_action(
                CommandAction::InitiateEmergencyCall,
            ));
        }

        let mut calls = self.calls.lock().unwrap();
        log::debug!("[CallService] Calls before dial: {:?}", calls);
        if calls.iter().any(|c| c.state == CallState::Dialing) {
            return ExecutionResult::Handled(HandledCommand::default()); // No response, just ignore
        }

        let mut did_hold = false;
        for call in calls.iter_mut() {
            if call.state == CallState::Active {
                call.state = CallState::Held;
                did_hold = true;
            }
        }

        let number_str = String::from_utf8(number.to_vec()).unwrap();
        calls.push(CallStatus {
            state: CallState::Dialing,
            direction: CallDirection::Outgoing,
            is_voice_mode: true,
            is_multi_party: false,
            number: number_str.clone(),
            peer_id: None,
        });
        log::debug!("[CallService] Calls after dial: {:?}", calls);

        let action = if did_hold {
            CommandAction::InitiateCallAndHold(number_str)
        } else {
            CommandAction::InitiateCall(number_str)
        };

        ExecutionResult::Handled(HandledCommand::ok_with_action(action))
    }

    pub fn handle_answer(&self, context: &ModemImpl) -> ExecutionResult {
        log::debug!("[CallService] Answering call");
        let mut calls = self.calls.lock().unwrap();
        log::debug!("[CallService] Calls before answer: {:?}", calls);
        if let Some(call) = calls.iter_mut().find(|c| c.state == CallState::Alerting) {
            call.state = CallState::Active;
            log::debug!("[CallService] Calls after answer: {:?}", calls);
            return ExecutionResult::Handled(HandledCommand::ok_with_action(
                CommandAction::AnswerCall(context.id),
            ));
        }
        ExecutionResult::Handled(HandledCommand::error())
    }

    pub fn handle_hangup(&self, context: &ModemImpl) -> ExecutionResult {
        let mut calls = self.calls.lock().unwrap();
        if calls.is_empty() {
            return ExecutionResult::Handled(HandledCommand::error());
        }
        calls.clear();
        ExecutionResult::Handled(HandledCommand::ok_with_action(CommandAction::HangupCall(
            context.id,
        )))
    }

    pub fn handle_call_hold(&self, op: u8) -> ExecutionResult {
        log::debug!("[CallService] Call hold operation: {}", op);
        let mut calls = self.calls.lock().unwrap();
        log::debug!("[CallService] Calls before hold op: {:?}", calls);
        if op == 2 {
            let active_pos = calls.iter().position(|c| c.state == CallState::Active);
            let held_pos = calls.iter().position(|c| c.state == CallState::Held);

            if let (Some(ap), Some(hp)) = (active_pos, held_pos) {
                let active_peer = calls[ap].peer_id;
                let held_peer = calls[hp].peer_id;
                calls[ap].state = CallState::Held;
                calls[hp].state = CallState::Active;
                log::debug!("[CallService] Calls after hold op: {:?}", calls);
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
        let calls = self.calls.lock().unwrap();
        let mut responses = Vec::new();
        for (i, call) in calls.iter().enumerate() {
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
            log::debug!("[CallService] Query current calls response: {}", response);
            responses.push(response);
        }
        responses.push("OK\r\n".to_string());
        ExecutionResult::Handled(HandledCommand { responses, action: None })
    }

    pub fn handle_remote_call(&self, number: &[u8]) -> ExecutionResult {
        let number_str = String::from_utf8(number.to_vec()).unwrap();
        let mut calls = self.calls.lock().unwrap();
        calls.push(CallStatus {
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

    pub fn handle_set_mute(&self, mute: u8) -> ExecutionResult {
        let mut self_mute = self.mute.lock().unwrap();
        *self_mute = mute == 1;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_mute(&self) -> ExecutionResult {
        let mute = self.mute.lock().unwrap();
        let response = format!("+CMUT: {}\r\n", if *mute { 1 } else { 0 });
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_send_dtmf(&self, _dtmf: &[u8]) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_emergency_mode(&self, mode: u8) -> ExecutionResult {
        let mut emergency_mode = self.emergency_mode.lock().unwrap();
        *emergency_mode = mode == 1;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_emergency_mode(&self) -> ExecutionResult {
        let emergency_mode = self.emergency_mode.lock().unwrap();
        let response = format!("+WSOS: {}\r\n", if *emergency_mode { 1 } else { 0 });
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }
}

impl CommandExecutor for CallService {
    fn execute(&self, context: &ModemImpl, command: &Command) -> ExecutionResult {
        match command {
            Command::Dial(number) => self.handle_dial(number),
            Command::Answer => self.handle_answer(context),
            Command::Hangup => self.handle_hangup(context),
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
