// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use netsim_model::{RadioTechnology, RegistrationStatus};
use tracing::{debug, error};

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
    types::{AT_OK, CmeError, CommandAction, ExecutionResult, ModemId},
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
    pub(crate) fn new(
        id: ModemId,
        profile: crate::config::SimProfile,
        sim_type: Option<i32>,
    ) -> Self {
        let enable_unsol = profile.enable_unsolicited_urcs.unwrap_or(true);
        Self {
            id,
            enable_unsolicited_urcs: enable_unsol,
            sim_service: SimService::new(&profile, sim_type),
            network_service: NetworkService::default(),
            sms_service: SmsService::default(),
            stk_service: StkService::default(),
            sup_service: SupService::default(),
            misc_service: MiscService::default(),
            call_service: CallService::default(),
            data_service: DataService::from_env(),
            phone_number: profile.msisdn.clone(),
            _state: State::Idle,
        }
    }

    pub fn trigger_incoming_call(&mut self, number: &str) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        let result = self.call_service.ring(number.to_string());
        if let ExecutionResult::Success(handled) = result {
            for response in handled.responses {
                if !response.is_empty() {
                    effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
                }
            }
            let clip = format!("+CLIP: \"{number}\",129,,,,0\r\n");
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
        debug!(
            "trigger_incoming_sms: id = {}, message_format = {:?}",
            self.id, self.sms_service.message_format
        );
        if self.sms_service.message_format == crate::sms_service::MessageFormat::Pdu {
            let pdu_hex = crate::pdu::create_deliver_pdu_ucs2(sender, text);
            return self.trigger_incoming_pdu(&pdu_hex);
        }
        let mut effects = Vec::new();
        // Format: +CMT: "<sender>",,"<timestamp>"\r\n<text>
        let timestamp = "22/01/01,12:00:00+00";
        let response = format!("+CMT: \"{sender}\",,\"{timestamp}\"\r\n{text}\r\n");
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
                let response = format!("+CMT: ,{tpdu_len}\r\n{pdu}\r\n");
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
                let response = format!("+CTZV: {zone}\r\n");
                effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
            }
        }
        effects
    }

    pub fn set_phone_number(&mut self, number: &str) {
        self.phone_number = number.to_string();
        self.sim_service.set_msisdn(number);
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

    pub fn set_sim_status(&mut self, present: bool) -> Vec<ModemEffect> {
        let changed = self.sim_service.set_present(present);
        let mut effects = Vec::new();
        if changed {
            if !present {
                effects.extend(self.set_voice_registration(RegistrationStatus::NotRegistered));
                effects.extend(self.set_data_registration(RegistrationStatus::NotRegistered));
                self.network_service.detach_network();
            } else {
                effects.push(ModemEffect::Schedule {
                    delay: Duration::from_millis(10),
                    event: ModemEvent::AttachNetwork,
                });
            }
        }
        effects
    }

    pub fn set_network_technology(&mut self, tech: RadioTechnology) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        if let Some(response) = self.network_service.set_network_technology(tech) {
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
                Some(ExecutionResult::Success(crate::types::HandledCommand::ok()))
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
            match result {
                ExecutionResult::Success(handled) => {
                    Self::append_handled_effects(&mut effects, handled);
                }
                ExecutionResult::Error => {
                    effects.push(ModemEffect::Response(b"ERROR\r\n".to_vec()));
                }
                ExecutionResult::ErrorWithUrc(urcs) => {
                    for urc in urcs {
                        effects.push(ModemEffect::Response(urc.into_bytes()));
                    }
                    effects.push(ModemEffect::Response(b"ERROR\r\n".to_vec()));
                }
                ExecutionResult::CmeError(err) => {
                    let resp = err.format_response(self.misc_service.cmee_mode());
                    effects.push(ModemEffect::Response(resp.into_bytes()));
                }
                ExecutionResult::CmeErrorWithUrc(err, urcs) => {
                    for urc in urcs {
                        effects.push(ModemEffect::Response(urc.into_bytes()));
                    }
                    let resp = err.format_response(self.misc_service.cmee_mode());
                    effects.push(ModemEffect::Response(resp.into_bytes()));
                }
                ExecutionResult::Unhandled => {
                    effects.push(ModemEffect::Response(b"ERROR\r\n".to_vec()));
                }
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
            return Vec::new();
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
                if self.sim_service.is_present() {
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
        if let ExecutionResult::Success(ref mut handled) = result {
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
                Ok((rem, command)) => {
                    if !rem.is_empty() {
                        error!(
                            "Failed to parse AT command {:?} (trailing garbage: {:?})",
                            String::from_utf8_lossy(cmd_bytes),
                            String::from_utf8_lossy(rem)
                        );
                        combined_responses.push(
                            CmeError::IncorrectParameters
                                .format_response(self.misc_service.cmee_mode()),
                        );
                        stop_chain = true;
                    } else {
                        match self.execute_and_schedule(&command, &mut combined_effects) {
                            ExecutionResult::Success(mut handled) => {
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
                            ExecutionResult::Error => {
                                combined_responses.push("ERROR\r\n".to_string());
                                stop_chain = true;
                            }
                            ExecutionResult::ErrorWithUrc(mut urcs) => {
                                combined_responses.append(&mut urcs);
                                combined_responses.push("ERROR\r\n".to_string());
                                stop_chain = true;
                            }
                            ExecutionResult::CmeError(err) => {
                                combined_responses
                                    .push(err.format_response(self.misc_service.cmee_mode()));
                                stop_chain = true;
                            }
                            ExecutionResult::CmeErrorWithUrc(err, mut urcs) => {
                                combined_responses.append(&mut urcs);
                                combined_responses
                                    .push(err.format_response(self.misc_service.cmee_mode()));
                                stop_chain = true;
                            }
                            ExecutionResult::Unhandled => {
                                error!("Unhandled command: {:?}", command);
                                combined_responses.push("ERROR\r\n".to_string());
                                stop_chain = true;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to parse AT command {:?}: {:?}",
                        String::from_utf8_lossy(cmd_bytes),
                        e
                    );
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
        } else if trimmed.eq_ignore_ascii_case(b"OK") || trimmed.eq_ignore_ascii_case(b"ERROR") {
            // Ignore echoed responses/URCs
            debug!("Ignoring echoed response/URC: {:?}", std::str::from_utf8(trimmed));
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
