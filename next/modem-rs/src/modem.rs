// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Write, sync::Arc, time::Duration};

use netsim_model::{Quirks, RadioTechnology, RegistrationStatus};
use tracing::{debug, error};

use crate::{
    call_service::CallService,
    config::SimProfile,
    constants::CALL_RING_TIMEOUT,
    data_service::DataService,
    misc_service::MiscService,
    network_service::{NetworkCommand, NetworkService},
    parser::Command,
    sim_service::SimService,
    sms_service::SmsService,
    stk_service::StkService,
    sup_service::SupService,
    time::Clock,
    types::{
        AT_OK, CmeError, CommandAction, CopsMode, ExecutionResult, HandledCommand, ModemError,
        ModemId, NumberPresentation, Parsable, PhoneNumber, RadioPowerLevel,
        RegistrationUnsolicitedMode, Response,
    },
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
    pub quirks: Quirks,
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

#[derive(Debug)]
pub enum ModemEffect {
    Action(CommandAction),
    Schedule { delay: Duration, event: ModemEvent },
    Response(Vec<u8>),
}

impl ModemImpl {
    pub(crate) fn new(
        id: ModemId,
        profile: SimProfile,
        quirks: Quirks,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let enable_unsol = profile.enable_unsolicited_urcs.unwrap_or(true);
        let home_plmn = profile.home_plmn();
        let mut sim_service = SimService::new();
        sim_service.load_profile(&profile);
        let mut misc_service = MiscService::new(clock);
        if quirks.auto_ctzv {
            misc_service.set_ctzv_mode(true);
        }
        Self {
            id,
            enable_unsolicited_urcs: enable_unsol,
            sim_service,
            network_service: NetworkService::new(quirks, home_plmn),
            sms_service: SmsService::default(),
            stk_service: StkService::new(profile.stk.clone()),
            sup_service: SupService::default(),
            misc_service,
            call_service: CallService::default(),
            data_service: DataService::from_env(),
            quirks,
            _state: State::Idle,
        }
    }

    pub fn trigger_incoming_call(
        &mut self,
        number: Option<&PhoneNumber>,
        number_presentation: NumberPresentation,
        peer_id: Option<ModemId>,
    ) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        let call_res = self.call_service.ring(number.cloned(), number_presentation, peer_id);
        let result: ExecutionResult = call_res.into();
        if let ExecutionResult::Success(handled) = result {
            for response in handled.responses {
                let resp_str = response.to_string();
                if !resp_str.is_empty() {
                    effects.push(ModemEffect::Response(resp_str.into_bytes()));
                }
            }
            if self.sup_service.clip_enabled() {
                let val = number_presentation.format_number(number);
                let mode = if number_presentation == NumberPresentation::Allowed && number.is_none()
                {
                    NumberPresentation::NotAvailable as u8
                } else {
                    number_presentation as u8
                };
                let number_str = val.number.strip_prefix('+').unwrap_or(val.number);
                let clip = format!("+CLIP: \"{number_str}\",{},,,,{mode}\r\n", val.toa);
                effects.push(ModemEffect::Response(clip.into_bytes()));
            }

            effects.push(ModemEffect::Schedule {
                delay: CALL_RING_TIMEOUT,
                event: ModemEvent::CallRingTimeout { call_token: 1 },
            });
        }
        effects
    }

    pub fn trigger_remote_answer(&mut self) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        if self.call_service.remote_answer() {
            effects.push(ModemEffect::Response(AT_OK.to_vec()));
        }
        effects
    }

    pub fn trigger_remote_hold(&mut self, on_hold: bool) -> Vec<ModemEffect> {
        self.call_service.trigger_remote_hold(on_hold);
        Vec::new()
    }

    pub fn trigger_remote_hangup(&mut self) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        self.call_service.receive_hangup();
        // Goldfish RIL uses RING as universal URC to trigger callRing/callStateChanged
        // for remote call teardown
        effects.push(ModemEffect::Response(b"RING\r\n".to_vec()));
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
                let omit_cmt_leading_comma = self.quirks.goldfish_ril_37_or_earlier;
                let comma = if omit_cmt_leading_comma { "" } else { "," };
                let response = format!("+CMT: {comma}{tpdu_len}\r\n{pdu}\r\n");
                effects.push(ModemEffect::Response(response.as_bytes().to_vec()));
            }
        }
        effects
    }

    pub fn trigger_network_time_update(&mut self) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        let response = self.misc_service.current_time_update();
        effects.push(ModemEffect::Response(response.to_string().into_bytes()));
        effects
    }

    pub fn set_phone_number(&mut self, number: &str) {
        let phone = PhoneNumber::parse(number.as_bytes()).map(|(_, p)| p).ok();
        self.sim_service.set_msisdn(phone.as_ref());
    }

    pub fn phone_number(&self) -> Option<PhoneNumber> {
        self.sim_service.get_msisdn()
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

    pub(crate) fn set_sim_status(&mut self, present: bool) -> Vec<ModemEffect> {
        if present && !self.sim_service.is_provisioned() {
            return Vec::new();
        }
        let changed = self.sim_service.set_present(present);
        let mut effects = Vec::new();
        if changed {
            if !present {
                effects.extend(self.set_voice_registration(RegistrationStatus::NotRegistered));
                effects.extend(self.set_data_registration(RegistrationStatus::NotRegistered));
                self.network_service.detach_network();
                self.network_service.set_home_plmn(None);
                self.stk_service = StkService::default();
                self.call_service.calls.clear();
            } else {
                let home_plmn = self.sim_service.home_plmn();
                self.network_service.set_home_plmn(home_plmn);
                if home_plmn.is_some() {
                    effects.push(ModemEffect::Schedule {
                        delay: Duration::from_millis(10),
                        event: ModemEvent::AttachNetwork,
                    });
                }
            }
            if !self.quirks.goldfish_ril_37_or_earlier
                && let Some(urc) = self.sim_service.get_cpin_urc()
            {
                effects.push(ModemEffect::Response(urc.as_bytes().to_vec()));
            }
        }
        effects
    }

    /// Ejects the SIM tray, cutting electrical power to the card while
    /// preserving non-volatile data on the card.
    pub(crate) fn eject_sim(&mut self) -> Vec<ModemEffect> {
        self.set_sim_status(false)
    }

    /// Re-inserts the SIM tray with the existing provisioned card.
    ///
    /// Fails if no SIM card is provisioned in the tray.
    pub(crate) fn reinsert_sim(&mut self) -> Result<Vec<ModemEffect>, ModemError> {
        if !self.sim_service.is_provisioned() {
            return Err(ModemError::InvalidConfig(
                "Cannot re-insert: SIM tray is empty".to_string(),
            ));
        }
        Ok(self.set_sim_status(true))
    }

    /// Removes and unprovisions the SIM card completely, wiping the filesystem
    /// and credentials.
    pub(crate) fn remove_sim(&mut self) -> Vec<ModemEffect> {
        let effects = self.set_sim_status(false);
        self.sim_service.remove_sim();
        effects
    }

    /// Synchronizes STK service and network registration state after a SIM
    /// change (profile switch or card insertion).
    fn sync_network_after_sim_change(&mut self, profile: &SimProfile) -> Vec<ModemEffect> {
        let mut effects = Vec::new();
        let home_plmn = profile.home_plmn();

        self.stk_service = StkService::new(profile.stk.clone());
        self.network_service.set_home_plmn(home_plmn);

        // Reset network registration to trigger fresh attachment to new home PLMN
        effects.extend(self.set_voice_registration(RegistrationStatus::NotRegistered));
        effects.extend(self.set_data_registration(RegistrationStatus::NotRegistered));
        self.network_service.detach_network();

        if home_plmn.is_some() {
            effects.push(ModemEffect::Schedule {
                delay: Duration::from_millis(10),
                event: ModemEvent::AttachNetwork,
            });
        }

        if !self.quirks.goldfish_ril_37_or_earlier
            && let Some(urc) = self.sim_service.get_cpin_urc()
        {
            effects.push(ModemEffect::Response(urc.as_bytes().to_vec()));
        }

        effects
    }

    /// Switches the active SIM profile, rebuilding the SIM filesystem and
    /// resynchronizing network operator, MSISDN, STK, and registration
    /// states.
    pub(crate) fn switch_sim_profile(&mut self, profile: SimProfile) -> Vec<ModemEffect> {
        self.sim_service.load_profile(&profile);
        self.sync_network_after_sim_change(&profile)
    }

    /// Inserts a SIM card into the modem by applying the provided profile.
    /// Fails if a SIM card is already provisioned.
    pub(crate) fn insert_sim(
        &mut self,
        profile: SimProfile,
    ) -> Result<Vec<ModemEffect>, ModemError> {
        if !self.sim_service.insert_sim(&profile) {
            return Err(ModemError::InvalidConfig("SIM card is already inserted".to_string()));
        }

        Ok(self.sync_network_after_sim_change(&profile))
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

                let sms_res = if store {
                    self.sms_service.handle_store_sms(&mut self.sim_service, pdu)
                } else {
                    let sender = self.phone_number();
                    let sender_str = sender.as_ref().map(|n| n.as_str()).unwrap_or("");
                    self.sms_service.handle_sms_body(pdu, sender_str)
                };

                let exec_res: ExecutionResult = sms_res.into();

                // Clear waiting state
                self.sms_service.waiting_for_pdu_len = None;
                self.sms_service.waiting_for_pdu_store = false;

                Some(exec_res)
            } else if command_bytes.contains(&0x1b) {
                // ESC
                // Abort
                self.sms_service.waiting_for_pdu_len = None;
                self.sms_service.waiting_for_pdu_store = false;
                Some(ExecutionResult::Success(HandledCommand::ok()))
            } else {
                None // Return None to wait for more data if the buffer is incomplete.
            }
        } else {
            None
        };

        if let Some(result) = sms_pdu_action {
            let mut effects = Vec::new();
            match result {
                ExecutionResult::Success(handled) => {
                    let mut combined = String::new();
                    for r in &handled.responses {
                        write!(combined, "{r}").unwrap();
                    }
                    if !combined.is_empty() {
                        effects.push(ModemEffect::Response(combined.into_bytes()));
                    }
                    for act in handled.actions {
                        effects.push(ModemEffect::Action(act));
                    }
                }
                ExecutionResult::Error { cme, urcs } => {
                    let mut combined = String::new();
                    for r in &urcs {
                        write!(combined, "{r}").unwrap();
                    }
                    let err_str = match cme {
                        Some(err) => err.format_response(self.misc_service.cmee_mode()),
                        None => "ERROR\r\n".to_string(),
                    };
                    combined.push_str(&err_str);
                    effects.push(ModemEffect::Response(combined.into_bytes()));
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
                let res = self.call_service.handle_ring_timeout(self.id, call_token);
                let result: ExecutionResult = res.into();
                if let ExecutionResult::Success(handled) = result {
                    for action in handled.actions {
                        effects.push(ModemEffect::Action(action));
                    }
                }
            }
            ModemEvent::AttachNetwork => {
                if self.sim_service.is_present() {
                    let mut responses = self.network_service.attach_network();
                    if self.misc_service.ctzv_enabled() && self.enable_unsolicited_urcs {
                        responses.push(self.misc_service.current_time_update().to_string());
                    }
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

    pub fn get_sms_count(&self) -> u32 {
        (self.sim_service.get_sms_count() + self.sms_service.get_sms_count()) as u32
    }

    pub fn is_ringing(&self) -> bool {
        self.call_service.has_incoming() || self.call_service.has_alerting()
    }

    pub fn get_active_calls(&self) -> Vec<String> {
        self.call_service
            .calls
            .iter()
            .filter(|c| c.state == crate::call_service::CallState::Active)
            .map(|c| {
                c.number.as_ref().map(|n: &PhoneNumber| n.as_str().to_string()).unwrap_or_default()
            })
            .collect()
    }

    pub fn set_operator(&mut self, operator: &str) -> Vec<ModemEffect> {
        let mode = if operator.is_empty() { CopsMode::Automatic } else { CopsMode::Manual };
        let oper = if operator.is_empty() { None } else { Some(operator.as_bytes()) };
        let mut effects = Vec::new();
        if let Ok(Some(crate::network_service::NetworkResponse::Urcs(urcs))) =
            self.network_service.set_operator_manual(mode, oper)
        {
            effects.extend(
                urcs.into_iter().map(|u| ModemEffect::Response(u.to_string().into_bytes())),
            );
        }
        effects
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
            if let Command::Network(NetworkCommand::SetRadioPower(RadioPowerLevel::Full)) = command
            {
                effects.push(ModemEffect::Schedule {
                    delay: std::time::Duration::from_millis(10),
                    event: ModemEvent::AttachNetwork,
                });
            }
            let mode_active = match command {
                Command::Network(
                    NetworkCommand::SetVoiceNetworkRegistration(m)
                    | NetworkCommand::SetDataNetworkRegistration(m)
                    | NetworkCommand::SetLteNetworkRegistration(m),
                ) => *m != RegistrationUnsolicitedMode::Disable,
                _ => false,
            };
            if mode_active && !self.network_service.is_attached() {
                effects.push(ModemEffect::Schedule {
                    delay: std::time::Duration::from_millis(10),
                    event: ModemEvent::AttachNetwork,
                });
            }
            for action in std::mem::take(&mut handled.actions) {
                effects.push(ModemEffect::Action(action));
            }
        }
        result
    }

    /// Executes a list of chained commands sequentially, halting on error and
    /// merging responses.
    fn execute_chained_commands(&mut self, sub_commands: &[Vec<u8>]) -> Vec<ModemEffect> {
        let mut combined_responses = String::new();
        let mut combined_effects = Vec::new();
        let mut stop_chain = false;

        for (i, cmd_bytes) in sub_commands.iter().enumerate() {
            let is_last = i == sub_commands.len() - 1;
            match Command::parse(cmd_bytes) {
                Ok((rem, command)) => {
                    let clean_rem = if rem == b"0"
                        && matches!(
                            command,
                            Command::Call(crate::call_service::CallCommand::Hangup)
                        ) {
                        b""
                    } else {
                        rem
                    };
                    if !clean_rem.is_empty() {
                        error!(
                            "Failed to parse AT command {:?} (trailing garbage: {:?})",
                            String::from_utf8_lossy(cmd_bytes),
                            String::from_utf8_lossy(rem)
                        );
                        write!(
                            combined_responses,
                            "{}",
                            CmeError::IncorrectParameters
                                .format_response(self.misc_service.cmee_mode())
                        )
                        .unwrap();
                        stop_chain = true;
                    } else {
                        let exec_res = self.execute_and_schedule(&command, &mut combined_effects);
                        match exec_res {
                            ExecutionResult::Success(handled) => {
                                let mut responses = handled.responses;
                                let success = responses.last() == Some(&Response::Ok);
                                if !is_last && success {
                                    responses.pop();
                                }
                                for r in responses {
                                    write!(combined_responses, "{r}").unwrap();
                                }
                                if !success {
                                    stop_chain = true;
                                }
                            }
                            ExecutionResult::Error { cme, urcs } => {
                                for r in urcs {
                                    write!(combined_responses, "{r}").unwrap();
                                }
                                let err_str = match cme {
                                    Some(err) => err.format_response(self.misc_service.cmee_mode()),
                                    None => "ERROR\r\n".to_string(),
                                };
                                combined_responses.push_str(&err_str);
                                stop_chain = true;
                            }
                            ExecutionResult::Unhandled => {
                                combined_responses.push_str("ERROR\r\n");
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
                    combined_responses.push_str("ERROR\r\n");
                    stop_chain = true;
                }
            }
            if stop_chain {
                break;
            }
        }

        if !combined_responses.is_empty() {
            combined_effects.insert(0, ModemEffect::Response(combined_responses.into_bytes()));
        }
        combined_effects
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::Sim(c) => self.sim_service.execute(c),
            Command::Call(c) => self.call_service.execute(
                c,
                self.id,
                &mut self.data_service,
                &self.sim_service,
                self.sup_service.clir_mode(),
            ),
            Command::Sms(c) => self.sms_service.execute(c, &mut self.sim_service),
            Command::Network(c) => self.network_service.execute(c, self.enable_unsolicited_urcs),
            Command::Data(c) => self.data_service.execute(c),
            Command::Misc(c) => self.misc_service.execute(c),
            Command::Sup(c) => self.sup_service.execute(c, &mut self.sim_service),
            Command::Stk(c) => self.stk_service.execute(c, &mut self.sim_service),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::SimProfile, time::SystemClock};

    #[test]
    fn test_execute_chained_commands_parse_error() {
        let mut modem =
            ModemImpl::new(1, SimProfile::default(), Quirks::default(), Arc::new(SystemClock));
        // We pass a command Y that returns Err on Command::parse(Y).
        // Since Y does not start with AT or RING, and we bypass split_chained_commands,
        // we can pass it directly to execute_chained_commands.
        let sub_commands = vec![b"INVALID".to_vec()];
        let effects = modem.execute_chained_commands(&sub_commands);

        assert_eq!(effects.len(), 1);
        if let ModemEffect::Response(resp) = &effects[0] {
            assert_eq!(resp, b"ERROR\r\n");
        } else {
            panic!("Expected Response effect");
        }
    }

    #[test]
    fn test_trigger_incoming_call_presentation_not_available() {
        let mut modem =
            ModemImpl::new(1, SimProfile::default(), Quirks::default(), Arc::new(SystemClock));
        // Enable CLIP via AT command
        modem.execute_chained_commands(&[b"AT+CLIP=1".to_vec()]);

        let phone = PhoneNumber::new("123456");
        let effects =
            modem.trigger_incoming_call(Some(&phone), NumberPresentation::NotAvailable, None);

        assert_eq!(effects.len(), 3);

        if let ModemEffect::Response(resp) = &effects[0] {
            assert_eq!(std::str::from_utf8(resp).unwrap(), "RING\r\n");
        } else {
            panic!("Expected Response effect for RING");
        }

        if let ModemEffect::Response(resp) = &effects[1] {
            assert_eq!(std::str::from_utf8(resp).unwrap(), "+CLIP: \"\",129,,,,2\r\n");
        } else {
            panic!("Expected Response effect for CLIP");
        }
    }
}
