// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use tracing::debug;

use crate::{
    parser::Command,
    types::{AT_OK, CommandAction, ExecutionResult, HandledCommand, ModemId},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CallDirection {
    Outgoing = 0,
    Incoming = 1,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CallState {
    Active = 0,
    Held = 1,
    Dialing = 2,
    Alerting = 3,
    Incoming = 4,
    Waiting = 5,
}

impl CallState {
    fn is_waiting(self) -> bool {
        matches!(self, CallState::Waiting | CallState::Incoming)
    }

    fn should_promote(self, has_waiting: bool) -> bool {
        self.is_waiting() || (self == CallState::Held && !has_waiting)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallStatus {
    pub id: u8,
    pub state: CallState,
    pub direction: CallDirection,
    pub is_voice_mode: bool,
    pub is_multi_party: bool,
    pub number: String,
    pub peer_id: Option<ModemId>,
}

#[derive(Debug, PartialEq)]
pub enum CallResponse {
    Ring,
    CurrentCalls(Vec<CallStatus>),
    Mute(bool),
    EmergencyMode(bool),
    WithAction(CommandAction),
    Empty,
}

impl std::fmt::Display for CallResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallResponse::Ring => write!(f, "RING\r\n"),
            CallResponse::CurrentCalls(calls) => {
                for call in calls {
                    let toa = if call.number.starts_with('+') { 145 } else { 129 };
                    write!(
                        f,
                        "+CLCC: {},{},{},{},{},\"{}\",{toa}\r\n",
                        call.id,
                        call.direction as u8,
                        call.state as u8,
                        if call.is_voice_mode { 0 } else { 1 },
                        call.is_multi_party as u8,
                        call.number,
                    )?;
                }
                Ok(())
            }
            CallResponse::Mute(mute) => {
                write!(f, "+CMUT: {}\r\n", if *mute { 1 } else { 0 })
            }
            CallResponse::EmergencyMode(mode) => {
                write!(f, "+WSOS: {}\r\n", if *mode { 1 } else { 0 })
            }
            CallResponse::WithAction(_) => Ok(()),
            CallResponse::Empty => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallError {
    Error,
    Unhandled,
}

pub type CallResult = Result<Option<CallResponse>, CallError>;

impl From<CallResult> for ExecutionResult {
    fn from(res: CallResult) -> Self {
        match res {
            Ok(opt_resp) => match opt_resp {
                Some(resp @ CallResponse::Ring) => ExecutionResult::Success(HandledCommand {
                    responses: vec![resp.to_string()],
                    action: None,
                }),
                Some(CallResponse::Empty) => ExecutionResult::Success(HandledCommand::default()),
                Some(CallResponse::WithAction(action)) => {
                    ExecutionResult::Success(HandledCommand::ok_with_action(action))
                }
                Some(resp) => {
                    let mut handled = HandledCommand::ok();
                    let resp_str = resp.to_string();
                    if !resp_str.is_empty() {
                        handled.responses.insert(0, resp_str);
                    }
                    ExecutionResult::Success(handled)
                }
                None => ExecutionResult::Success(HandledCommand::ok()),
            },
            Err(CallError::Error) => ExecutionResult::Error,
            Err(CallError::Unhandled) => ExecutionResult::Unhandled,
        }
    }
}

// Holds all state related to the call service.
#[derive(Default)]
pub struct CallService {
    pub calls: Vec<CallStatus>,
    mute: bool,
    emergency_mode: bool,
}

impl CallService {
    fn has_waiting_call(&self) -> bool {
        self.calls.iter().any(|c| c.state.is_waiting())
    }

    fn has_call_in_state(&self, state: CallState) -> bool {
        self.calls.iter().any(|c| c.state == state)
    }

    fn has_call(&self, id: u8) -> bool {
        self.calls.iter().any(|c| c.id == id)
    }

    // Allocates a stable 1-indexed ID for AT+CLCC/CHLD; N is small (typically <= 7)
    // due to network limits.
    fn allocate_id(&self) -> Option<u8> {
        (1..=u8::MAX).find(|&id| !self.has_call(id))
    }

    pub fn add_call(
        &mut self,
        state: CallState,
        direction: CallDirection,
        number: String,
        peer_id: Option<ModemId>,
    ) -> Option<u8> {
        let id = self.allocate_id()?;
        self.calls.push(CallStatus {
            id,
            state,
            direction,
            is_voice_mode: true,
            is_multi_party: false,
            number,
            peer_id,
        });
        Some(id)
    }

    pub fn remove_call(&mut self, id: u8) {
        if let Some(pos) = self.calls.iter().position(|c| c.id == id) {
            self.calls.remove(pos);
        }
    }

    // --- Helper methods for external services ---

    pub fn receive_hangup(&mut self) {
        self.calls.clear();
    }

    pub fn receive_hangup_from_peer_id(&mut self, peer_id: ModemId) {
        self.calls.retain(|c| c.peer_id != Some(peer_id));
    }

    pub fn handle_ring_timeout(&mut self, _call_token: u32) {
        self.calls.retain(|c| c.state != CallState::Incoming);
    }

    pub fn ring(&mut self, number: String) -> CallResult {
        if self.add_call(CallState::Incoming, CallDirection::Incoming, number, None).is_none() {
            return Err(CallError::Error);
        }
        Ok(Some(CallResponse::Ring))
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
        self.has_call_in_state(CallState::Dialing)
    }

    pub fn is_alerting(&self) -> bool {
        self.has_call_in_state(CallState::Alerting)
    }

    pub fn is_active(&self) -> bool {
        self.has_call_in_state(CallState::Active)
    }

    pub fn is_held(&self) -> bool {
        self.has_call_in_state(CallState::Held)
    }

    pub fn is_incoming(&self) -> bool {
        self.has_call_in_state(CallState::Incoming)
    }

    pub fn connect(&mut self, _modem_id: ModemId, peer_id: ModemId) -> Option<Vec<u8>> {
        debug!("[CallService] Connecting to peer {peer_id}");
        debug!("[CallService] Calls before connect: {:?}", self.calls);

        if let Some(call) = self.calls.iter_mut().find(|c| c.state == CallState::Dialing) {
            call.state = CallState::Active;
            call.peer_id = Some(peer_id);
            debug!("[CallService] Connected as caller. Calls: {:?}", self.calls);
            return Some(AT_OK.to_vec());
        }

        if let Some(call) =
            self.calls.iter_mut().find(|c| c.state == CallState::Active && c.peer_id.is_none())
        {
            call.peer_id = Some(peer_id);
            debug!("[CallService] Connected as callee (peer_id set). Calls: {:?}", self.calls);
            return None;
        }

        debug!("[CallService] Connect failed. Calls: {:?}", self.calls);
        None
    }

    // --- Pure command handlers ---

    pub fn handle_dial(&mut self, number: &[u8]) -> CallResult {
        debug!("[CallService] Dialing number: {}", String::from_utf8_lossy(number));
        // GPRS dial commands (e.g. ATD*99#) are GPRS packet-data requests and should
        // fall back to DataService.
        if crate::constants::is_gprs_dial(number) {
            return Err(CallError::Unhandled);
        }

        let Some(dial_str) = parse_number(number) else {
            return Err(CallError::Error);
        };
        let mut is_emergency = false;
        let clean_number = if let Some(pos) = dial_str.find('@') {
            is_emergency = true;
            // TODO: Support emergency categories and CLIR suffixes (e.g. @1,#I) currently
            // they are discarded.
            &dial_str[..pos]
        } else {
            let stripped = dial_str.trim_end_matches([';', 'i', 'I']);
            if stripped == "911" {
                is_emergency = true;
            }
            stripped
        };

        // '+' is only valid as the very first character (international prefix)
        let is_valid = !clean_number.is_empty()
            && clean_number.bytes().enumerate().all(|(i, b)| match b {
                b'+' => i == 0,
                b'0'..=b'9' | b'*' | b'#' => true,
                _ => false,
            });

        if !is_valid {
            return Err(CallError::Error);
        }

        if is_emergency {
            return Ok(Some(CallResponse::WithAction(CommandAction::InitiateEmergencyCall)));
        }

        debug!("[CallService] Calls before dial: {:?}", self.calls);
        if self.is_dialing() {
            return Ok(Some(CallResponse::Empty));
        }

        let mut did_hold = false;
        for call in self.calls.iter_mut() {
            if call.state == CallState::Active {
                call.state = CallState::Held;
                did_hold = true;
            }
        }

        let clean_number_str = clean_number.to_string();
        if self
            .add_call(CallState::Dialing, CallDirection::Outgoing, clean_number_str.clone(), None)
            .is_none()
        {
            return Err(CallError::Error);
        }
        debug!("[CallService] Calls after dial: {:?}", self.calls);

        let action = if did_hold {
            CommandAction::InitiateCallAndHold(clean_number_str)
        } else {
            CommandAction::InitiateCall(clean_number_str)
        };

        Ok(Some(CallResponse::WithAction(action)))
    }

    pub fn handle_answer(&mut self, id: ModemId) -> CallResult {
        debug!("[CallService] Answering call");
        debug!("[CallService] Calls before answer: {:?}", self.calls);
        if let Some(call) =
            self.calls.iter_mut().find(|c| c.state == CallState::Alerting || c.state.is_waiting())
        {
            call.state = CallState::Active;
            debug!("[CallService] Calls after answer: {:?}", self.calls);
            return Ok(Some(CallResponse::WithAction(CommandAction::AnswerCall(id))));
        }
        Err(CallError::Error)
    }

    pub fn handle_hangup(&mut self, id: ModemId) -> CallResult {
        if self.is_idle() {
            return Err(CallError::Error);
        }
        self.calls.clear();
        Ok(Some(CallResponse::WithAction(CommandAction::HangupCall(id))))
    }

    pub fn handle_call_hold(&mut self, raw_chld_op: u8, id: ModemId) -> CallResult {
        debug!("[CallService] Call hold operation: {raw_chld_op}");
        debug!("[CallService] Calls before hold op: {:?}", self.calls);

        let (op, index) = if raw_chld_op >= 100 {
            (raw_chld_op / 100, Some(raw_chld_op % 100))
        } else if raw_chld_op >= 10 {
            (raw_chld_op / 10, Some(raw_chld_op % 10))
        } else {
            (raw_chld_op, None)
        };

        // Validate index for ops that require it (1 and 2)
        if index.is_some_and(|idx| (op == 1 || op == 2) && !self.has_call(idx)) {
            return Err(CallError::Error);
        }

        match op {
            0 => {
                let prev_len = self.calls.len();
                self.calls.retain(|c| c.state != CallState::Held && !c.state.is_waiting());
                if self.calls.len() < prev_len {
                    return Ok(Some(CallResponse::WithAction(CommandAction::HangupCall(id))));
                }
            }
            1 => {
                let prev_len = self.calls.len();
                if let Some(idx) = index {
                    self.remove_call(idx);
                } else {
                    self.calls.retain(|c| c.state != CallState::Active);
                    let has_waiting = self.has_waiting_call();
                    for call in self.calls.iter_mut() {
                        if call.state.should_promote(has_waiting) {
                            call.state = CallState::Active;
                        }
                    }
                }
                if self.calls.len() < prev_len {
                    return Ok(Some(CallResponse::WithAction(CommandAction::HangupCall(id))));
                }
            }
            2 => {
                if let Some(idx) = index {
                    for call in self.calls.iter_mut() {
                        if call.id == idx {
                            call.state = CallState::Active;
                            call.is_multi_party = false;
                        } else if call.state == CallState::Active {
                            call.state = CallState::Held;
                        }
                    }
                } else {
                    let has_waiting = self.has_waiting_call();
                    for call in self.calls.iter_mut() {
                        if call.state == CallState::Active {
                            call.state = CallState::Held;
                        } else if call.state.should_promote(has_waiting) {
                            call.state = CallState::Active;
                        }
                    }
                }
            }
            3 => {
                if !self.is_active() || !self.is_held() {
                    return Err(CallError::Error);
                }
                for call in self.calls.iter_mut() {
                    if call.state == CallState::Held {
                        call.state = CallState::Active;
                    }
                    if call.state == CallState::Active {
                        call.is_multi_party = true;
                    }
                }
            }
            4 => {
                // ECT: Connect remote parties and disconnect us.
                // TODO: Support true ECT by bridging peers in the simulator, but it is
                // currently unsupported in the Android Emulator RIL.
                // For now, we hang up to avoid leaking state.
                return self.handle_hangup(id);
            }
            _ => return Err(CallError::Error),
        }
        debug!("[CallService] Calls after hold op: {:?}", self.calls);
        Ok(None)
    }

    pub fn handle_query_current_calls(&self) -> CallResult {
        if self.calls.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CallResponse::CurrentCalls(self.calls.clone())))
        }
    }

    pub fn handle_remote_call(&mut self, number: &[u8]) -> CallResult {
        let Some(number_str) = parse_number(number) else {
            return Err(CallError::Error);
        };
        if self
            .add_call(CallState::Incoming, CallDirection::Incoming, number_str.clone(), None)
            .is_none()
        {
            return Err(CallError::Error);
        }
        Ok(Some(CallResponse::WithAction(CommandAction::InitiateRemoteCall(number_str))))
    }

    pub fn handle_set_mute(&mut self, mute: u8) -> CallResult {
        self.mute = mute == 1;
        Ok(None)
    }

    pub fn handle_query_mute(&self) -> CallResult {
        Ok(Some(CallResponse::Mute(self.mute)))
    }

    pub fn handle_send_dtmf(&self, dtmf: &[u8]) -> CallResult {
        debug!("[CallService] Send DTMF: {}", String::from_utf8_lossy(dtmf));

        if std::str::from_utf8(dtmf).is_ok_and(is_valid_dtmf_format) {
            Ok(None)
        } else {
            Err(CallError::Error)
        }
    }

    pub fn handle_set_emergency_mode(&mut self, mode: u8) -> CallResult {
        self.emergency_mode = mode == 1;
        Ok(None)
    }

    pub fn handle_query_emergency_mode(&self) -> CallResult {
        Ok(Some(CallResponse::EmergencyMode(self.emergency_mode)))
    }

    pub fn execute(&mut self, command: &Command, id: ModemId) -> ExecutionResult {
        let res = match command {
            Command::Dial(number) => self.handle_dial(number),
            Command::Answer => self.handle_answer(id),
            Command::Hangup => self.handle_hangup(id),
            Command::CallHold(op) => self.handle_call_hold(*op, id),
            Command::QueryCurrentCalls => self.handle_query_current_calls(),
            Command::Ring => self.ring("".to_string()),
            Command::RemoteCall(number) => self.handle_remote_call(number),
            Command::SetMute(mute) => self.handle_set_mute(*mute),
            Command::QueryMute => self.handle_query_mute(),
            Command::SendDtmf(dtmf) => self.handle_send_dtmf(dtmf),
            Command::SetEmergencyMode(mode) => self.handle_set_emergency_mode(*mode),
            Command::QueryEmergencyMode => self.handle_query_emergency_mode(),
            _ => Err(CallError::Unhandled),
        };
        res.into()
    }
}

fn parse_number(number: &[u8]) -> Option<String> {
    std::str::from_utf8(number).ok().map(|s| s.trim().to_string())
}

fn is_valid_dtmf_format(dtmf_str: &str) -> bool {
    let (digit_part, duration_part) =
        dtmf_str.split_once(',').map(|(d, dur)| (d, Some(dur))).unwrap_or((dtmf_str, None));

    if digit_part.len() != 1 {
        return false;
    }
    let digit = digit_part.as_bytes()[0];
    if !matches!(digit, b'0'..=b'9' | b'#' | b'*' | b'A'..=b'D' | b'a'..=b'd') {
        return false;
    }

    if duration_part.is_some_and(|dur| dur.is_empty() || !dur.chars().all(|c| c.is_ascii_digit())) {
        return false;
    }

    true
}
