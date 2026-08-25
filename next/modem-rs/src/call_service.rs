// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use modem_rs_derive::CommandParser;
use tracing::debug;

use crate::{
    data_service::DataService,
    parser::parse_raw_data,
    sim_service::SimService,
    types::{
        CallHoldAction, CallHoldParam, ClirMode, CmeError, CommandAction, DialArgs,
        ExecutionResult, ModemId, NumberPresentation, Parsable, PhoneNumber,
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

/// Standard 3GPP TS 27.007 §7.18 (+CLCC) voice call states.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CallState {
    /// Connected voice call.
    Active = 0,
    /// Call on hold.
    Held = 1,
    /// Outgoing call dialing.
    Dialing = 2,
    /// Outgoing call ringing remote peer.
    Alerting = 3,
    /// Inbound call ringing.
    Incoming = 4,
    /// Inbound call waiting while another call is active.
    Waiting = 5,
}

impl CallState {
    pub fn is_outbound(self) -> bool {
        matches!(self, CallState::Dialing | CallState::Alerting)
    }

    fn is_inbound(self) -> bool {
        matches!(self, CallState::Incoming | CallState::Waiting)
    }

    fn is_foreground(self) -> bool {
        matches!(self, CallState::Active | CallState::Dialing | CallState::Alerting)
    }

    fn is_answerable(self) -> bool {
        matches!(self, CallState::Alerting | CallState::Incoming | CallState::Waiting)
    }

    /// Prioritizes answering inbound calls over resuming held calls during
    /// accept/swap operations.
    fn should_promote_to_active(self, has_inbound: bool) -> bool {
        self.is_inbound() || (self == CallState::Held && !has_inbound)
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
    WithActions(Vec<CommandAction>),
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
                        "+CLCC: {},{},{},{},{},{},{}\r\n",
                        call.id,
                        call.direction as u8,
                        call.state as u8,
                        if call.is_voice_mode { 0 } else { 1 },
                        call.is_multi_party as u8,
                        val.number.strip_prefix('+').unwrap_or(val.number),
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
            CallResponse::WithActions(_) => Ok(()),
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

    fn add_call(
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

    fn remove_call(&mut self, id: u8) {
        if let Some(pos) = self.calls.iter().position(|c| c.id == id) {
            self.calls.remove(pos);
        }
    }

    /// Returns true if this call is currently part of an active or held
    /// multi-party conference with >= 2 remote participants in the same
    /// state, per 3GPP TS 22.084.
    fn is_multiparty_call(&self, call: &CallStatus) -> bool {
        call.is_multi_party
            && self.calls.iter().filter(|c| c.state == call.state && c.is_multi_party).count() >= 2
    }

    // --- Helper methods for external services ---

    pub fn receive_hangup(&mut self) {
        self.calls.clear();
    }

    pub fn receive_hangup_from_peer_id(&mut self, peer_id: ModemId) {
        self.calls.retain(|c| c.peer_id != Some(peer_id));
    }

    pub fn handle_ring_timeout(&mut self, id: ModemId, _call_token: u32) -> CallResult {
        let mut actions = Vec::new();
        self.calls.retain(|call| {
            if call.state.is_inbound() {
                if let Some(peer_id) = call.peer_id {
                    actions.push(CommandAction::HangupCall { initiator: id, target_peer: peer_id });
                }
                false
            } else {
                true
            }
        });
        if !actions.is_empty() { Ok(Some(CallResponse::WithActions(actions))) } else { Ok(None) }
    }

    pub fn ring(
        &mut self,
        number: Option<PhoneNumber>,
        number_presentation: NumberPresentation,
        peer_id: Option<ModemId>,
    ) -> CallResult {
        let initial_state =
            if self.has_active() { CallState::Waiting } else { CallState::Incoming };
        if self
            .add_call(initial_state, CallDirection::Incoming, number, number_presentation, peer_id)
            .is_none()
        {
            return Err(ExecutionResult::error());
        }
        Ok(Some(CallResponse::Ring))
    }

    pub fn receive_hold(&mut self, peer_id: ModemId) {
        debug!("[CallService] Receiving hold from {:?}", peer_id);
        if let Some(call) = self.calls.iter_mut().find(|c| c.peer_id == Some(peer_id)) {
            call.state = CallState::Held;
        }
    }

    pub fn receive_resume(&mut self, peer_id: ModemId) {
        debug!("[CallService] Receiving resume from {:?}", peer_id);
        if let Some(call) = self.calls.iter_mut().find(|c| c.peer_id == Some(peer_id)) {
            call.state = CallState::Active;
        }
    }

    pub fn trigger_remote_hold(&mut self, on_hold: bool) {
        let target_state = if on_hold { CallState::Active } else { CallState::Held };
        let new_state = if on_hold { CallState::Held } else { CallState::Active };
        // Prioritize independent calls created directly via CLI/RPC (peer_id is None)
        if let Some(call) =
            self.calls.iter_mut().find(|c| c.state == target_state && c.peer_id.is_none())
        {
            call.state = new_state;
        } else if let Some(call) = self.calls.iter_mut().find(|c| c.state == target_state) {
            call.state = new_state;
        }
    }

    pub fn remote_answer(&mut self) -> bool {
        if let Some(call) = self.calls.iter_mut().find(|c| c.state.is_outbound()) {
            call.state = CallState::Active;
            return true;
        }
        false
    }

    fn is_idle(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn has_alerting(&self) -> bool {
        self.has_call_in_state(CallState::Alerting)
    }

    pub fn has_outbound(&self) -> bool {
        self.calls.iter().any(|c| c.state.is_outbound())
    }

    fn has_inbound(&self) -> bool {
        self.calls.iter().any(|c| c.state.is_inbound())
    }

    fn has_active(&self) -> bool {
        self.has_call_in_state(CallState::Active)
    }

    fn has_held(&self) -> bool {
        self.has_call_in_state(CallState::Held)
    }

    pub fn has_incoming(&self) -> bool {
        self.has_call_in_state(CallState::Incoming)
    }

    pub fn connect(&mut self, _modem_id: ModemId, peer_id: ModemId) -> Option<Vec<u8>> {
        debug!("[CallService] Connecting to peer {peer_id}");
        debug!("[CallService] Calls before connect: {:?}", self.calls);

        if let Some(call) = self.calls.iter_mut().find(|c| c.state.is_outbound()) {
            call.state = CallState::Active;
            call.peer_id = Some(peer_id);
            debug!("[CallService] Connected as caller.");
            return Some(b"RING\r\n".to_vec());
        }

        if let Some(call) = self.calls.iter_mut().find(|c| {
            c.state == CallState::Active && (c.peer_id.is_none() || c.peer_id == Some(peer_id))
        }) {
            call.peer_id = Some(peer_id);
            debug!("[CallService] Connected as callee (peer_id set). Calls: {:?}", self.calls);
            return None;
        }

        debug!("[CallService] Connect failed. Calls: {:?}", self.calls);
        None
    }

    // --- Command handlers ---

    fn handle_dial(
        &mut self,
        args: DialArgs,
        id: ModemId,
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

        let result = self.handle_voice_dial(id, args, sim_service, clir_mode);
        result.into()
    }

    fn handle_voice_dial(
        &mut self,
        id: ModemId,
        args: DialArgs,
        sim_service: &SimService,
        clir_mode: ClirMode,
    ) -> CallResult {
        if args.is_emergency {
            return Ok(Some(CallResponse::WithActions(vec![CommandAction::InitiateEmergencyCall])));
        }

        if !sim_service.is_fdn_allowed(&args.number) {
            return Err(ExecutionResult::cme_error(CmeError::FixedDialNumberOnlyAllowed));
        }

        debug!("[CallService] Calls before dial: {:?}", self.calls);
        if self.has_outbound() {
            return Err(ExecutionResult::cme_error(CmeError::OperationNotAllowed));
        }

        let mut actions = Vec::new();
        for call in self.calls.iter_mut() {
            if call.state == CallState::Active {
                call.state = CallState::Held;
                if let Some(peer_id) = call.peer_id {
                    actions.push(CommandAction::HoldCall { holder: id, target: peer_id });
                }
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

        actions.push(CommandAction::InitiateCall(DialArgs {
            number: args.number,
            clir: call_clir,
            is_emergency: false,
        }));

        Ok(Some(CallResponse::WithActions(actions)))
    }

    fn handle_answer(&mut self, id: ModemId) -> CallResult {
        debug!("[CallService modem={id}] Answering call");
        debug!("[CallService modem={id}] Calls before answer: {:?}", self.calls);
        if let Some(call) = self.calls.iter_mut().find(|c| c.state.is_answerable()) {
            call.state = CallState::Active;
            debug!("[CallService modem={id}] Calls after answer: {:?}", self.calls);
            return Ok(Some(CallResponse::WithActions(vec![CommandAction::AnswerCall(id)])));
        }
        Err(ExecutionResult::error())
    }

    fn handle_hangup(&mut self, id: ModemId) -> CallResult {
        if self.is_idle() {
            return Err(ExecutionResult::error());
        }
        let actions: Vec<CommandAction> = self
            .calls
            .iter()
            .filter_map(|call| {
                call.peer_id.map(|peer_id| CommandAction::HangupCall {
                    initiator: id,
                    target_peer: peer_id,
                })
            })
            .collect();
        self.calls.clear();
        if !actions.is_empty() { Ok(Some(CallResponse::WithActions(actions))) } else { Ok(None) }
    }

    fn handle_call_hold(&mut self, chld: CallHoldParam, id: ModemId) -> CallResult {
        debug!("[CallService] Call hold operation: {chld:?}");
        debug!("[CallService] Calls before hold op: {:?}", self.calls);

        let (op, index) = (chld.op, chld.call_id);

        // Releasing when no calls exist is a no-op that succeeds with OK.
        if self.calls.is_empty()
            && (op == CallHoldAction::ReleaseHeld || op == CallHoldAction::ReleaseAndAccept)
        {
            return Ok(None);
        }

        // Validate index if specified for operations targeting a specific call
        if let Some(idx) = index
            && !self.has_call(idx)
        {
            return Err(ExecutionResult::error());
        }

        match op {
            CallHoldAction::ReleaseHeld => {
                let mut actions = Vec::new();
                let prev_len = self.calls.len();
                let has_inbound = self.has_inbound();
                self.calls.retain(|call| {
                    let should_drop = if has_inbound {
                        call.state.is_inbound()
                    } else {
                        call.state == CallState::Held
                    };
                    if should_drop {
                        if let Some(peer_id) = call.peer_id {
                            actions.push(CommandAction::HangupCall {
                                initiator: id,
                                target_peer: peer_id,
                            });
                        }
                        false
                    } else {
                        true
                    }
                });
                if self.calls.len() < prev_len {
                    return Ok(Some(CallResponse::WithActions(actions)));
                }
            }
            CallHoldAction::ReleaseAndAccept => {
                let mut actions = Vec::new();
                if let Some(idx) = index {
                    if let Some(call) = self.calls.iter().find(|c| c.id == idx)
                        && let Some(peer_id) = call.peer_id
                    {
                        actions.push(CommandAction::HangupCall {
                            initiator: id,
                            target_peer: peer_id,
                        });
                    }
                    self.remove_call(idx);
                } else {
                    let has_inbound = self.has_inbound();
                    self.calls.retain(|c| {
                        if c.state.is_foreground() {
                            if let Some(peer) = c.peer_id {
                                actions.push(CommandAction::HangupCall {
                                    initiator: id,
                                    target_peer: peer,
                                });
                            }
                            false
                        } else {
                            true
                        }
                    });
                    for call in self.calls.iter_mut() {
                        let was_inbound = call.state.is_inbound();
                        if call.state.should_promote_to_active(has_inbound) {
                            if was_inbound {
                                actions.push(CommandAction::AnswerCall(id));
                            } else if let Some(peer_id) = call.peer_id {
                                actions.push(CommandAction::ResumeCall {
                                    resumer: id,
                                    target: peer_id,
                                });
                            }
                            call.state = CallState::Active;
                        }
                    }
                }
                if !actions.is_empty() {
                    return Ok(Some(CallResponse::WithActions(actions)));
                }
            }
            CallHoldAction::HoldAndAccept => {
                let mut actions = Vec::new();
                if let Some(idx) = index {
                    for call in self.calls.iter_mut() {
                        if call.id == idx {
                            if call.state == CallState::Held
                                && let Some(peer_id) = call.peer_id
                            {
                                actions.push(CommandAction::ResumeCall {
                                    resumer: id,
                                    target: peer_id,
                                });
                            } else if call.state.is_inbound() {
                                actions.push(CommandAction::AnswerCall(id));
                            }
                            call.state = CallState::Active;
                            call.is_multi_party = false;
                        } else if call.state == CallState::Active {
                            if let Some(peer_id) = call.peer_id {
                                actions
                                    .push(CommandAction::HoldCall { holder: id, target: peer_id });
                            }
                            call.state = CallState::Held;
                        }
                    }
                } else {
                    let has_inbound = self.has_inbound();
                    for call in self.calls.iter_mut() {
                        if call.state == CallState::Active {
                            if let Some(peer_id) = call.peer_id {
                                actions
                                    .push(CommandAction::HoldCall { holder: id, target: peer_id });
                            }
                            call.state = CallState::Held;
                        } else {
                            let was_inbound = call.state.is_inbound();
                            if call.state.should_promote_to_active(has_inbound) {
                                if was_inbound {
                                    actions.push(CommandAction::AnswerCall(id));
                                } else if let Some(peer_id) = call.peer_id {
                                    actions.push(CommandAction::ResumeCall {
                                        resumer: id,
                                        target: peer_id,
                                    });
                                }
                                call.state = CallState::Active;
                            }
                        }
                    }
                }
                if !actions.is_empty() {
                    return Ok(Some(CallResponse::WithActions(actions)));
                }
            }
            CallHoldAction::Conference => {
                if !self.has_active() || !self.has_held() {
                    return Err(ExecutionResult::error());
                }
                let mut actions = Vec::new();
                for call in self.calls.iter_mut() {
                    match call.state {
                        CallState::Held => {
                            call.state = CallState::Active;
                            call.is_multi_party = true;
                            if let Some(peer_id) = call.peer_id {
                                actions.push(CommandAction::ResumeCall {
                                    resumer: id,
                                    target: peer_id,
                                });
                            }
                        }
                        CallState::Active => {
                            call.is_multi_party = true;
                        }
                        _ => {}
                    }
                }
                if !actions.is_empty() {
                    return Ok(Some(CallResponse::WithActions(actions)));
                }
            }
            CallHoldAction::Transfer => {
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

    fn handle_query_current_calls(&self) -> CallResult {
        if self.calls.is_empty() {
            Ok(None)
        } else {
            let calls = self
                .calls
                .iter()
                .cloned()
                .map(|mut c| {
                    c.is_multi_party = self.is_multiparty_call(&c);
                    c
                })
                .collect();
            Ok(Some(CallResponse::CurrentCalls(calls)))
        }
    }

    fn handle_remote_call(&mut self, number: PhoneNumber) -> CallResult {
        if self
            .add_call(
                CallState::Dialing,
                CallDirection::Outgoing,
                Some(number.clone()),
                NumberPresentation::Allowed,
                None,
            )
            .is_none()
        {
            return Err(ExecutionResult::error());
        }
        Ok(Some(CallResponse::WithActions(vec![CommandAction::InitiateRemoteCall(number)])))
    }

    fn handle_set_mute(&mut self, mute: u8) -> CallResult {
        if mute > 1 {
            return Err(ExecutionResult::error());
        }
        self.mute = mute == 1;
        Ok(None)
    }

    fn handle_query_mute(&self) -> CallResult {
        Ok(Some(CallResponse::Mute(self.mute)))
    }

    fn handle_send_dtmf(&self, dtmf: &[u8]) -> CallResult {
        debug!("[CallService] Send DTMF: {}", String::from_utf8_lossy(dtmf));

        if std::str::from_utf8(dtmf).is_ok_and(is_valid_dtmf_format) {
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    fn handle_set_emergency_mode(&mut self, mode: u8) -> CallResult {
        self.emergency_mode = mode == 1;
        Ok(None)
    }

    fn handle_query_emergency_mode(&self) -> CallResult {
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
                return self.handle_dial(args.clone(), id, data_service, sim_service, clir_mode);
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

    let clean_digit = digit_part.trim_matches('"');
    if clean_digit.len() != 1 {
        return false;
    }
    let digit = clean_digit.as_bytes()[0];
    if !matches!(digit, b'0'..=b'9' | b'#' | b'*' | b'A'..=b'D' | b'a'..=b'd') {
        return false;
    }

    if duration_part.is_some_and(|dur| dur.is_empty() || !dur.chars().all(|c| c.is_ascii_digit())) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{CallDirection, CallService, CallState};
    use crate::types::{NumberPresentation, PhoneNumber};

    #[test]
    fn test_clcc_not_available() {
        let mut service = CallService::default();
        service.add_call(
            CallState::Incoming,
            CallDirection::Incoming,
            Some(PhoneNumber::new("123456")),
            NumberPresentation::NotAvailable,
            None,
        );

        let calls_res = service.handle_query_current_calls().unwrap().unwrap();
        let formatted = calls_res.to_string();

        assert_eq!(formatted, "+CLCC: 1,1,4,0,0,,129\r\n");
    }
}
