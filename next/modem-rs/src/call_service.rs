// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use modem_rs_derive::CommandParser;
use tracing::debug;

use crate::{
    data_service::DataService,
    parser::parse_raw_data,
    sim_service::SimService,
    types::{
        AT_OK, CallHoldAction, CallHoldParam, ClirMode, CommandAction, DialArgs, ExecutionResult,
        ModemId, NumberPresentation, Parsable, PhoneNumber,
    },
};

/// Call service AT commands.
#[derive(Debug, PartialEq, Clone, CommandParser)]
pub enum CallCommand<'a> {
    #[command(tag = "ATD")]
    Dial(DialArgs),
    #[command(tag = "ATA")]
    Answer,
    #[command(tag = "ATH")]
    Hangup,
    #[command(tag = "AT+CHLD=")]
    CallHold(CallHoldParam),
    #[command(tag = "AT+CLCC")]
    QueryCurrentCalls,
    #[command(tag = "AT+CMUT=")]
    SetMute(u8),
    #[command(tag = "AT+CMUT?")]
    QueryMute,
    #[command(tag = "AT+VTS=")]
    SendDtmf(#[parser(parse_raw_data)] &'a [u8]),
    /// VENDOR: Set emergency mode
    #[command(tag = "AT+WSOS=")]
    SetEmergencyMode(u8),
    /// VENDOR: Query emergency mode
    #[command(tag = "AT+WSOS?")]
    QueryEmergencyMode,
    /// VENDOR: Remote call
    #[command(tag = "AT+REMOTECALL=")]
    RemoteCall(PhoneNumber),
    /// VENDOR: Ring indication
    #[command(tag = "RING")]
    Ring,
}

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
    pub number: Option<PhoneNumber>,
    pub peer_id: Option<ModemId>,
    pub number_presentation: NumberPresentation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
                    let val = call.number_presentation.format_number(call.number.as_ref());
                    write!(
                        f,
                        "+CLCC: {},{},{},{},{},\"{}\",{}\r\n",
                        call.id,
                        call.direction as u8,
                        call.state as u8,
                        if call.is_voice_mode { 0 } else { 1 },
                        call.is_multi_party as u8,
                        val.number,
                        val.toa,
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

type CallResult = Result<Option<CallResponse>, ExecutionResult>;

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
        number: Option<PhoneNumber>,
        number_presentation: NumberPresentation,
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
            number_presentation,
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

    pub fn ring(
        &mut self,
        number: Option<PhoneNumber>,
        number_presentation: NumberPresentation,
        peer_id: Option<ModemId>,
    ) -> CallResult {
        if self
            .add_call(
                CallState::Incoming,
                CallDirection::Incoming,
                number,
                number_presentation,
                peer_id,
            )
            .is_none()
        {
            return Err(ExecutionResult::error());
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

    pub fn handle_dial(
        &mut self,
        args: DialArgs,
        data_service: &mut DataService,
        sim_service: &SimService,
        clir_mode: ClirMode,
    ) -> ExecutionResult {
        debug!("[CallService] Dialing number: {}", args.number.as_str());
        // GPRS dial commands (e.g. ATD*99#) are GPRS packet-data requests and are
        // handled by DataService.
        if args.number.is_gprs_dial() {
            return data_service.handle_gprs_dial(args.number.as_str().as_bytes()).into();
        }

        let result = self.handle_voice_dial(args, sim_service, clir_mode);
        result.into()
    }

    fn handle_voice_dial(
        &mut self,
        args: DialArgs,
        _sim_service: &SimService,
        clir_mode: ClirMode,
    ) -> CallResult {
        if args.is_emergency {
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

        if self
            .add_call(
                CallState::Dialing,
                CallDirection::Outgoing,
                Some(args.number.clone()),
                NumberPresentation::Allowed,
                None,
            )
            .is_none()
        {
            return Err(ExecutionResult::error());
        }
        debug!("[CallService] Calls after dial: {:?}", self.calls);

        let call_clir =
            if args.clir == ClirMode::SubscriptionDefault { clir_mode } else { args.clir };

        let action = if did_hold {
            CommandAction::InitiateCallAndHold(DialArgs {
                number: args.number,
                clir: call_clir,
                is_emergency: false,
            })
        } else {
            CommandAction::InitiateCall(DialArgs {
                number: args.number,
                clir: call_clir,
                is_emergency: false,
            })
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
        Err(ExecutionResult::error())
    }

    pub fn handle_hangup(&mut self, id: ModemId) -> CallResult {
        if self.is_idle() {
            return Err(ExecutionResult::error());
        }
        self.calls.clear();
        Ok(Some(CallResponse::WithAction(CommandAction::HangupCall(id))))
    }

    pub fn handle_call_hold(&mut self, chld: CallHoldParam, id: ModemId) -> CallResult {
        debug!("[CallService] Call hold operation: {chld:?}");
        debug!("[CallService] Calls before hold op: {:?}", self.calls);

        let (op, index) = (chld.op, chld.call_id);

        // Validate index for ops that require it (1 and 2)
        if index.is_some_and(|idx| {
            (op == CallHoldAction::ReleaseActiveAcceptHeldOrWaiting
                || op == CallHoldAction::HoldActiveAcceptHeldOrWaiting)
                && !self.has_call(idx)
        }) {
            return Err(ExecutionResult::error());
        }

        match op {
            CallHoldAction::ReleaseHeldOrWaiting => {
                let prev_len = self.calls.len();
                self.calls.retain(|c| c.state != CallState::Held && !c.state.is_waiting());
                if self.calls.len() < prev_len {
                    return Ok(Some(CallResponse::WithAction(CommandAction::HangupCall(id))));
                }
            }
            CallHoldAction::ReleaseActiveAcceptHeldOrWaiting => {
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
            CallHoldAction::HoldActiveAcceptHeldOrWaiting => {
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
            CallHoldAction::AddHeld => {
                if !self.is_active() || !self.is_held() {
                    return Err(ExecutionResult::error());
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
            CallHoldAction::Ect => {
                // ECT: Connect remote parties and disconnect us.
                // TODO: Support true ECT by bridging peers in the simulator, but it is
                // currently unsupported in the Android Emulator RIL.
                // For now, we hang up to avoid leaking state.
                return self.handle_hangup(id);
            }
            CallHoldAction::UserToUserSignaling => {
                return Err(ExecutionResult::error());
            }
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

    pub fn handle_remote_call(&mut self, number: PhoneNumber) -> CallResult {
        if self
            .add_call(
                CallState::Incoming,
                CallDirection::Incoming,
                Some(number.clone()),
                NumberPresentation::Allowed,
                None,
            )
            .is_none()
        {
            return Err(ExecutionResult::error());
        }
        Ok(Some(CallResponse::WithAction(CommandAction::InitiateRemoteCall(number))))
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
            Err(ExecutionResult::error())
        }
    }

    pub fn handle_set_emergency_mode(&mut self, mode: u8) -> CallResult {
        self.emergency_mode = mode == 1;
        Ok(None)
    }

    pub fn handle_query_emergency_mode(&self) -> CallResult {
        Ok(Some(CallResponse::EmergencyMode(self.emergency_mode)))
    }

    pub fn execute<'a>(
        &mut self,
        command: &CallCommand<'a>,
        id: ModemId,
        data_service: &mut DataService,
        sim_service: &SimService,
        clir_mode: ClirMode,
    ) -> ExecutionResult {
        let res = match command {
            CallCommand::Dial(args) => {
                return self.handle_dial(args.clone(), data_service, sim_service, clir_mode);
            }
            CallCommand::Answer => self.handle_answer(id),
            CallCommand::Hangup => self.handle_hangup(id),
            CallCommand::CallHold(op) => self.handle_call_hold(*op, id),
            CallCommand::QueryCurrentCalls => self.handle_query_current_calls(),
            CallCommand::Ring => self.ring(None, NumberPresentation::Allowed, None),
            CallCommand::RemoteCall(number) => self.handle_remote_call(number.clone()),
            CallCommand::SetMute(mute) => self.handle_set_mute(*mute),
            CallCommand::QueryMute => self.handle_query_mute(),
            CallCommand::SendDtmf(dtmf) => self.handle_send_dtmf(dtmf),
            CallCommand::SetEmergencyMode(mode) => self.handle_set_emergency_mode(*mode),
            CallCommand::QueryEmergencyMode => self.handle_query_emergency_mode(),
        };
        res.into()
    }
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
