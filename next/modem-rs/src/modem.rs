// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use netsim_model::RegistrationStatus;
use tracing::error;

use crate::{
    call_service::CallService,
    constants::CALL_RING_TIMEOUT,
    data_service::DataService,
    misc_service::MiscService,
    network_service::NetworkService,
    parser::Command,
    sim_service::SimService,
    sms_service::SmsService,
    stk_service::StkService,
    sup_service::SupService,
    types::{AT_ERROR, AT_OK, CommandAction, ExecutionResult, ModemId},
};

/// Represents a single modem device.
pub struct ModemImpl {
    pub id: ModemId,
    pub enable_unsolicited_urcs: bool,
    pub sim_service: SimService,
    pub network_service: NetworkService,
    pub sms_service: SmsService,
    pub call_service: CallService,
    pub stk_service: StkService,
    pub sup_service: SupService,
    pub misc_service: MiscService,
    pub data_service: DataService,
    phone_number: String,
    _state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModemEvent {
    CallRingTimeout { call_token: u32 },
    AttachNetwork,
    TestEvent,
}

pub enum ModemEffect {
    Action(CommandAction),
    Schedule { delay: Duration, event: ModemEvent },
    Response(Vec<u8>),
}

impl ModemImpl {
    pub(crate) fn new(id: ModemId, profile: crate::config::SimProfile) -> Self {
        let enable_unsol = profile.enable_unsolicited_urcs.unwrap_or(true);
        Self {
            id,
            enable_unsolicited_urcs: enable_unsol,
            sim_service: SimService::new(&profile),
            network_service: NetworkService::default(),
            sms_service: SmsService::default(),
            stk_service: StkService::default(),
            sup_service: SupService::default(),
            misc_service: MiscService::default(),
            call_service: CallService::default(),
            data_service: DataService::default(),
            phone_number: "".to_string(),
            _state: State::Idle,
        }
    }

    pub fn trigger_incoming_call(&mut self, number: &str) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        let result = self.call_service.ring(number.to_string());
        if let ExecutionResult::Handled(handled) = result {
            for response in handled.responses {
                if !response.is_empty() {
                    effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
                }
            }
            let clip = format!("+CLIP: \"{}\",129,,,,0\r\n", number);
            effects.push(ModemEffect::Response(clip.as_bytes().to_vec()));

            effects.push(ModemEffect::Schedule {
                delay: CALL_RING_TIMEOUT,
                event: ModemEvent::CallRingTimeout { call_token: 1 },
            });
        }
        effects
    }

    pub fn trigger_remote_answer(&mut self) -> Vec<ModemEffect> {
        let mut effects = Vec::new(); // Was missing initialization in original block? No, Vec::new() was at end.
        if self.call_service.remote_answer() {
            effects.push(ModemEffect::Response(AT_OK.to_vec()));
        }
        effects
    }

    pub fn trigger_remote_hold(&mut self, on_hold: bool) -> Vec<ModemEffect> {
        if on_hold {
            self.call_service.receive_hold();
        } else {
            self.call_service.receive_resume();
        }
        Vec::new()
    }

    pub fn trigger_remote_hangup(&mut self) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        self.call_service.receive_hangup();
        effects.push(ModemEffect::Response(b"NO CARRIER\r\n".to_vec()));
        effects
    }

    pub fn trigger_incoming_sms(&mut self, sender: &str, text: &str) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        // Format: +CMT: "<sender>",,"<timestamp>"\r\n<text>
        let timestamp = "22/01/01,12:00:00+00";
        let response = format!("+CMT: \"{}\",,\"{}\"\r\n{}\r\n", sender, timestamp, text);
        effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
        effects
    }

    pub fn trigger_incoming_pdu(&mut self, pdu: &str) -> Vec<ModemEffect> {
        // Calculate TPDU length
        let mut effects = Vec::new();
        if let Ok(bytes) = hex::decode(pdu)
            && !bytes.is_empty()
        {
            let sca_len = bytes[0] as usize;
            if bytes.len() > 1 + sca_len {
                let tpdu_len = bytes.len() - 1 - sca_len;
                let response = format!("+CMT: ,{}\r\n{}\r\n", tpdu_len, pdu);
                effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
            }
        }
        effects
    }

    pub fn trigger_network_time_update(&mut self, time: &str) -> Vec<ModemEffect> {
        self.misc_service.set_time(time.to_string());
        let mut effects = Vec::new();
        // Extract timezone for +CTZV
        if let Some(pos) = time.rfind('+').or_else(|| time.rfind('-')) {
            // Basic check to avoid date separators if any
            if pos > 10 {
                let zone = &time[pos..];
                let response = format!("+CTZV: {}\r\n", zone);
                effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
            }
        }
        effects
    }

    pub fn set_phone_number(&mut self, number: &str) {
        self.phone_number = number.to_string();
    }

    pub fn phone_number(&self) -> String {
        self.phone_number.clone()
    }

    pub fn set_signal_strength(&mut self, rssi: u8, ber: u8) {
        self.network_service.set_signal_strength(rssi, ber);
    }

    pub fn set_voice_registration(&mut self, status: RegistrationStatus) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        if let Some(response) = self.network_service.set_voice_registration(status) {
            effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
        }
        effects
    }

    pub fn set_data_registration(&mut self, status: RegistrationStatus) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        if let Some(response) = self.network_service.set_data_registration(status) {
            effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
        }
        effects
    }

    // Removed set_waiting_for_sms_pdu, moved to SmsService

    /// Receives an AT command from the modem.
    pub fn receive_at_command(&mut self, command_bytes: &[u8]) -> Vec<ModemEffect> {
        // Check for SMS PDU submission first. This requires special state handling.
        let sms_pdu_action = if self.sms_service.waiting_for_pdu_len.is_some() {
            if command_bytes.ends_with(b"\x1a") {
                let pdu = &command_bytes[..command_bytes.len() - 1];
                let store = self.sms_service.waiting_for_pdu_store;

                let result = if store {
                    self.sms_service.handle_store_sms(&mut self.sim_service, pdu)
                } else {
                    self.sms_service.handle_sms_body(pdu)
                };

                // Clear waiting state
                self.sms_service.waiting_for_pdu_len = None;
                self.sms_service.waiting_for_pdu_store = false;

                Some(result)
            } else if command_bytes.contains(&0x1b) {
                // ESC
                // Abort
                self.sms_service.waiting_for_pdu_len = None;
                self.sms_service.waiting_for_pdu_store = false;
                Some(ExecutionResult::Handled(crate::types::HandledCommand::ok()))
            } else {
                None // Waiting for more data? Or just ignore for now if
                // incomplete? The emulator usually
                // sends full line/buffer.
            }
        } else {
            None
        };

        if let Some(result) = sms_pdu_action {
            let mut effects = Vec::new();
            if let ExecutionResult::Handled(handled) = result {
                Self::append_handled_effects(&mut effects, handled);
            }
            return effects;
        }

        let mut len = command_bytes.len();
        while len > 0 && (command_bytes[len - 1] == b'\r' || command_bytes[len - 1] == b'\n') {
            len -= 1;
        }
        let command_clean = &command_bytes[..len];

        let sub_commands = split_chained_commands(command_clean);
        if sub_commands.is_empty() {
            return vec![ModemEffect::Response(AT_ERROR.to_vec())];
        }
        self.execute_chained_commands(&sub_commands)
    }

    pub fn tick(&mut self) -> Vec<ModemEffect> {
        Vec::new()
    }

    pub fn handle_event(&mut self, event: ModemEvent) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        match event {
            ModemEvent::TestEvent => {
                effects.push(ModemEffect::Response(b"TEST_EVENT_FIRED\r\n".to_vec()));
            }
            ModemEvent::CallRingTimeout { call_token } => {
                self.call_service.handle_ring_timeout(call_token);
            }
            ModemEvent::AttachNetwork => {
                let responses = self.network_service.attach_network();
                if self.enable_unsolicited_urcs {
                    let combined = responses
                        .iter()
                        .filter(|r| !r.is_empty())
                        .cloned()
                        .collect::<Vec<String>>()
                        .join("");
                    if !combined.is_empty() {
                        effects.push(ModemEffect::Response(combined.into_bytes()));
                    }
                }
            }
        }
        effects
    }

    pub fn get_sms_count(&self) -> usize {
        self.sim_service.get_sms_count() + self.sms_service.get_sms_count()
    }

    pub fn is_ringing(&self) -> bool {
        self.call_service.is_incoming() || self.call_service.is_alerting()
    }

    pub fn get_active_calls(&self) -> Vec<String> {
        self.call_service
            .calls
            .iter()
            .filter(|c| c.state == crate::call_service::CallState::Active)
            .map(|c| c.number.clone())
            .collect()
    }

    pub fn call_service(&self) -> &CallService {
        &self.call_service
    }

    /// Executes a single command and schedules any associated side-effects
    /// (e.g. network attachment).
    fn execute_and_schedule(
        &mut self,
        command: &Command,
        effects: &mut Vec<ModemEffect>,
    ) -> ExecutionResult {
        let mut result = self.execute(command);
        if let ExecutionResult::Handled(ref mut handled) = result {
            if let Command::SetRadioPower(1) = command {
                effects.push(ModemEffect::Schedule {
                    delay: std::time::Duration::from_millis(10),
                    event: ModemEvent::AttachNetwork,
                });
            }
            let mode_active = match command {
                Command::SetVoiceNetworkRegistration(m)
                | Command::SetDataNetworkRegistration(m)
                | Command::SetLteNetworkRegistration(m) => *m > 0,
                _ => false,
            };
            if mode_active && !self.network_service.is_attached() {
                effects.push(ModemEffect::Schedule {
                    delay: std::time::Duration::from_millis(10),
                    event: ModemEvent::AttachNetwork,
                });
            }
            if let Some(action) = handled.action.take() {
                effects.push(ModemEffect::Action(action));
            }
        }
        result
    }

    /// Executes a list of chained commands sequentially, halting on error and
    /// merging responses.
    fn execute_chained_commands(&mut self, sub_commands: &[Vec<u8>]) -> Vec<ModemEffect> {
        let mut combined_responses = Vec::new();
        let mut combined_effects = Vec::new();
        let mut stop_chain = false;

        for (i, cmd_bytes) in sub_commands.iter().enumerate() {
            let is_last = i == sub_commands.len() - 1;
            match Command::parse(cmd_bytes) {
                Ok((_, command)) => {
                    match self.execute_and_schedule(&command, &mut combined_effects) {
                        ExecutionResult::Handled(mut handled) => {
                            let success =
                                handled.responses.last().map(|s| s.as_str()) == Some("OK\r\n");
                            if !is_last && success {
                                handled.responses.pop();
                            }
                            combined_responses.append(&mut handled.responses);
                            if !success {
                                stop_chain = true;
                            }
                        }
                        ExecutionResult::Unhandled => {
                            error!("Unhandled command: {:?}", command);
                            combined_responses.push("ERROR\r\n".to_string());
                            stop_chain = true;
                        }
                    }
                }
                Err(_) => {
                    combined_responses.push("ERROR\r\n".to_string());
                    stop_chain = true;
                }
            }
            if stop_chain {
                break;
            }
        }

        let combined = combined_responses
            .iter()
            .filter(|r| !r.is_empty())
            .cloned()
            .collect::<Vec<String>>()
            .join("");
        if !combined.is_empty() {
            combined_effects.insert(0, ModemEffect::Response(combined.into_bytes()));
        }
        combined_effects
    }

    pub fn execute(&mut self, command: &Command) -> crate::types::ExecutionResult {
        let result = self.misc_service.execute(command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        // SMS Service needs SimService
        let result = self.sms_service.execute(command, &mut self.sim_service);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        // Call Service needs ModemId
        let result = self.call_service.execute(command, self.id);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.data_service.execute(command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.network_service.execute(command, self.enable_unsolicited_urcs);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.sim_service.execute(command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.stk_service.execute(command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.sup_service.execute(command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        ExecutionResult::Unhandled
    }

    fn append_handled_effects(
        effects: &mut Vec<ModemEffect>,
        handled: crate::types::HandledCommand,
    ) {
        let combined = handled
            .responses
            .iter()
            .filter(|r| !r.is_empty())
            .cloned()
            .collect::<Vec<String>>()
            .join("");
        if !combined.is_empty() {
            effects.push(ModemEffect::Response(combined.into_bytes()));
        }
        if let Some(action) = handled.action {
            effects.push(ModemEffect::Action(action));
        }
    }
}

/// Splits a chained command line by semicolons (ignoring them inside quotes)
/// and normalizes prefixes.
fn split_chained_commands(input: &[u8]) -> Vec<Vec<u8>> {
    let mut commands = Vec::new();
    let mut current = Vec::new();
    let mut in_quotes = false;
    for &b in input {
        if b == b'"' {
            in_quotes = !in_quotes;
            current.push(b);
        } else if b == b';' && !in_quotes {
            if !current.is_empty() {
                commands.push(current);
                current = Vec::new();
            }
        } else {
            current.push(b);
        }
    }
    if !current.is_empty() {
        commands.push(current);
    }

    let mut processed = Vec::new();
    for cmd in commands {
        let trimmed = trim_slice(&cmd);
        if trimmed.is_empty() {
            continue;
        }
        if starts_with_ignore_case(trimmed, b"AT") || starts_with_ignore_case(trimmed, b"RING") {
            processed.push(trimmed.to_vec());
        } else {
            let mut new_cmd = b"AT".to_vec();
            new_cmd.extend_from_slice(trimmed);
            processed.push(new_cmd);
        }
    }
    processed
}

/// Trims leading and trailing ASCII whitespace from a byte slice.
fn trim_slice(s: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < s.len() && s[start].is_ascii_whitespace() {
        start += 1;
    }
    let mut end = s.len();
    while end > start && s[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &s[start..end]
}

/// Checks if a byte slice starts with a prefix, ignoring ASCII case.
fn starts_with_ignore_case(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}
